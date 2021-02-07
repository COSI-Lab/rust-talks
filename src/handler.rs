use crate::{ws, Client, Clients, Result};
use futures::SinkExt;
use uuid::Uuid;
use warp::{Reply, hyper::StatusCode, reply::json, ws::Message};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Debug)]
pub struct RegisterResponse {
    url: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "event")]
pub enum Event {
    Create { name: String, talk_type: TalkType, desc: String },
    Hide { id: usize },
    UnHide { id: usize },
    Delete { id: usize }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TalkType {
    ForumTopic,
    LightningTalk,
    ProjectUpdate,
    Announcement,
    AfterMeetingSlot,
}

// Adds a new client to the clients map and returns URL for websocket connection
pub async fn register_handler(clients: Clients) -> Result<impl Reply> {
    // 128 bit UUID, a colision might as well be impossible
    let uuid = Uuid::new_v4().simple().to_string();

    // Adds new client to map
    clients.write().await.insert(
        uuid.clone(),
        Client {
            sender: None,
        }
    );

    // Returns url for websocket connection
    Ok(json(&RegisterResponse {
        url: format!("ws://127.0.0.1:8000/ws/{}", uuid),
    }))
}

// Removes client from the clients map
pub async fn unregister_handler(id: String, clients: Clients) -> Result<impl Reply> {
    clients.write().await.remove(&id);
    Ok(StatusCode::OK)
}

// Publishes incoming events too all clients
pub async fn publish_handler(body: Event, clients: Clients) -> Result<impl Reply> {
    println!("/publish received");

    // Reserialize body as a string
    match serde_json::to_string(&body) {
        Ok(body) => {
            // For each client send the message
            clients.write().await.iter_mut()
                .for_each(|(id, client)| {
                    println!("{}: {}", id, body);
                    if let Some(sender) = &mut client.sender {
                        println!("Feeding");
                        let _ = sender.feed(Ok(Message::text(&body)));
                        println!("Flushing");
                        let _ = sender.flush();
                    }
                });

            Ok(StatusCode::OK)
        }
        Err(_) => {
            Ok(StatusCode::BAD_REQUEST)
        }
    }
}

// Turns HTTP request into a websocket
pub async fn ws_handler(ws: warp::ws::Ws, id: String, clients: Clients) -> Result<impl Reply> {
    let client = clients.read().await.get(&id).cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| ws::client_connection(socket, id, clients, c))),
        None => Err(warp::reject::not_found()),
    }
}

// Always returns 200
pub async fn health_handler() -> Result<impl Reply> {
    let event = Event::Hide{id: 1};

    match serde_json::to_string(&event) {
        Ok(b) => { Ok(b) }
        Err(e) => { Ok(e.to_string()) }
    }
}