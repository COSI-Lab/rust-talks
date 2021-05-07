use std::net::IpAddr;
use askama::Template;
use uuid::Uuid;
use warp::{Rejection, Reply, hyper::StatusCode, reply::{html, json}};
use serde::{Serialize, Deserialize};

use crate::{Clients, client::{Client, client_connection}, db::DBManager, model::Talk};

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
                let octects = ip.octets();
                octects[0] == 128 && octects[1] == 153
            }
            IpAddr::V6(ip) => {
                let segments = ip.segments();
                segments[0] == 0x2605 && segments[1] == 0x6480 && segments[3] == 0xc051
            }
        };
    }

    // Adds new client to map
    clients.write().await.insert(
        id.clone(),
        Client {
            sender: None,
            authenticated,
            second_chance: true,
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
            let mut str = talks.iter()
                .filter_map(|talk| serde_json::to_string(talk).ok())
                .fold(String::from("["), |a, b| a + &b + ",");            
            
            // remove the last `,`
            str.pop();
            str.push(']');

            Ok(str)
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
