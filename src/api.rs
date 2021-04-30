use std::net::{IpAddr, SocketAddr};
use askama::Template;
use uuid::Uuid;
use warp::{Rejection, Reply, hyper::StatusCode, reply::{html, json}};
use serde::{Serialize, Deserialize};
use futures::{StreamExt, channel::mpsc};
use warp::ws::WebSocket;

use crate::{Client, Clients, db::DBManager, events::{EventRequest, send_events, process_event}, model::Talk};

#[derive(Template)]
#[template(path = "index.j2")]
struct IndexTemplate {
    talks: Vec<Talk>
}

// Return the talks homepage
pub async fn welcome_handler(db: DBManager) -> Result<impl Reply, Rejection> {
    match db.list_visible_talks() {
        Ok(talks) => { 
            let template = IndexTemplate {
                talks
            };

            Ok(html(template.render().unwrap()))
        }
        Err(err) => { Err(err.into()) }
    }
}

// Always returns 200
pub async fn health_handler(clients: Clients) -> Result<impl Reply, Rejection>  {
    let clients = clients.read().await;

    Ok(format!("Open Connections: {}\n", clients.len()))
}

#[derive(Serialize, Debug)]
pub struct RegisterResponse {
    id: String,
    authenticated: bool,
}

// Adds a new client to the clients map and returns URL for websocket connection
pub async fn register_handler(addr: Option<IpAddr>, clients: Clients) -> Result<impl Reply, Rejection> {
    // 128 bit UUID, a colision should be impossible
    let id = Uuid::new_v4().simple().to_string();

    let mut authenticated: bool = false;

    if let Some(addr) = addr {
        // Authenticate the user based on their ip address
        authenticated = match addr {
            IpAddr::V4(ip) => { 
                // check the ip is in '128.153.0.0/16'
                let octects = ip.octets();
                octects[0] == 128 && octects[1] == 153
            }
            IpAddr::V6(_) => { false }
        };
    }

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
        id,
        authenticated 
    }))
}

#[derive(Deserialize, Debug)]
pub struct AuthenticateRequest {
    id: String,
    password: String,
}

pub async fn authenticate(request: AuthenticateRequest, clients: Clients) -> Result<impl Reply, Rejection> {
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

pub async fn visible_talks(db: DBManager) -> Result<impl Reply, Rejection> {
    match db.list_visible_talks() {
        Ok(talks) => { 
            Ok(talks.iter()
                .filter_map(|talk| serde_json::to_string(talk).ok())
                .fold(String::from("["), |a, b| a + &b + ",") + "]"
            )
        }
        Err(err) => { Err(err.into()) }
    }
}

// Turns HTTP request into a websocket
pub async fn ws_handler(ws: warp::ws::Ws, id: String, clients: Clients, db: DBManager) -> Result<impl Reply, Rejection> {
    let client = clients.read().await.get(&id).cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| client_connection(socket, id, clients, c, db))),
        None => Err(warp::reject::not_found()),
    }
}

#[derive(Serialize, Deserialize)]
pub struct Password {
    password: String
}

// Handles the connection to the websocket
pub async fn client_connection(ws: WebSocket, id: String, clients: Clients, mut client: Client, db: DBManager) {
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
                    let response = process_event(event, &db);
                    send_events(clients.clone(), response).await;
                }
            }
        }
    }

    clients.write().await.remove(&id);
    println!("{} disconnected", id);
}
