use std::net::{IpAddr, SocketAddr};
use askama::Template;
use uuid::Uuid;
use warp::{Reply, hyper::StatusCode, reply::{html, json}};
use serde::{Serialize, Deserialize};
use futures::{SinkExt, StreamExt};
use futures::channel::mpsc;
use warp::ws::WebSocket;

use crate::{Client, Clients, DB, EventQueue, HOST, Result, events::{EventRequest, Talk}};

#[derive(Template)]
#[template(path = "index.j2")]
struct IndexTemplate {
    talks: Vec<Talk>
}

// Return the talks homepage
pub async fn welcome_handler(db: DB) -> Result<impl Reply> {
    let mut talks: Vec<Talk> = Vec::new();

    db.read().await.iter()
        .filter(|talk| talk.is_visible)
        .for_each(|talk| talks.push(talk.clone()));

    let template = IndexTemplate {
        talks
    };

    Ok(html(template.render().unwrap()))
}

// Always returns 200
pub async fn health_handler() -> Result<impl Reply> {
    let x = Password { password: String::from("password") };
    match serde_json::to_string(&x) {
        Ok(s) => { Ok(s) }
        Err(_) => { Ok(String::from("not ok")) }
    }
}

#[derive(Serialize, Debug)]
pub struct RegisterResponse {
    url: String,
    id: String,
    authenticated: bool,
}

// Adds a new client to the clients map and returns URL for websocket connection
pub async fn register_handler(addr: SocketAddr, clients: Clients) -> Result<impl Reply> {
    // 128 bit UUID, a colision might as well be impossible
    let id = Uuid::new_v4().simple().to_string();

    // Authenticate the user based on their ip address
    let authenticated: bool = match addr.ip() {
        IpAddr::V4(ip) => { 
            // check the ip is in '128.153.0.0/16'
            let octects = ip.octets();
            octects[0] == 128 && octects[1] == 153
        }
        IpAddr::V6(_) => { false }
    };

    // Adds new client to map
    clients.write().await.insert(
        id.clone(),
        Client {
            sender: None,
            authenticated,
        }
    );

    // Returns url for websocket connection
    Ok(json(&RegisterResponse { 
        url: format!("ws://{}/ws/{}", HOST, id),
        id,
        authenticated 
    }))
}

#[derive(Deserialize, Debug)]
pub struct AuthenticateRequest {
    id: String,
    password: String,
}

pub async fn authenticate(request: AuthenticateRequest, clients: Clients) -> Result<impl Reply> {
    // Check if the client is already authenticated
    if request.password == "conway" {
        let mut writer = clients.write().await;
        match writer.get_mut(&request.id) {
            Some(client) => {
                // set authenticated flag
                client.authenticated = true;
                return Ok(StatusCode::OK);
            }
            None => {
                return Ok(StatusCode::BAD_REQUEST);
            }
        }
    }

    return Ok(StatusCode::FORBIDDEN);
}

pub async fn visible_talks(db: DB) -> Result<impl Reply> {
    let result = db.read().await.iter()
        .filter(|talk| talk.is_visible)
        .filter_map(|talk| serde_json::to_string(talk).ok())
        .fold(String::from("["), |a, b| a + &b + ",");

    Ok(result + "]")
}

// Turns HTTP request into a websocket
pub async fn ws_handler(ws: warp::ws::Ws, id: String, clients: Clients, queue: EventQueue) -> Result<impl Reply> {
    let client = clients.read().await.get(&id).cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| client_connection(socket, id, clients, c, queue))),
        None => Err(warp::reject::not_found()),
    }
}

#[derive(Serialize, Deserialize)]
pub struct Password {
    password: String
}

// Handles the connection to the websocket
pub async fn client_connection(ws: WebSocket, id: String, clients: Clients, mut client: Client, queue: EventQueue) {
    let (client_ws_sender, mut client_ws_rcv) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded();

    // Create a new task that just forwards all messages from client_rcv into the websocket
    tokio::task::spawn(client_rcv.forward(client_ws_sender));

    // Update client
    client.sender = Some(client_sender);
    clients.write().await.insert(id.clone(), client);

    println!("{} connected", id);

    // Red messages forever
    while let Some(result) = client_ws_rcv.next().await {
        // Checks if the client is authenticated
        if clients.read().await.get(&id).map_or(false, |client| client.authenticated) {
            // Read message
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    eprintln!("error receiving ws message for id: {}): {}", id.clone(), e);
                    break;
                }
            };

            // If message is a string
            if let Ok(str) = msg.to_str() {
                // Parse message as event
                if let Ok(event) = serde_json::from_str::<EventRequest>(&str) {
                    let mut text = str.to_string();
                    text.push('\n');

                    let _ = queue.write().await.queue.send((event, text)).await;
                }
            }
        }
    }

    clients.write().await.remove(&id);
    println!("{} disconnected", id);
}
