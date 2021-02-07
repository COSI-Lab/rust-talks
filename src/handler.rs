use crate::{Client, Clients, Result};
use uuid::Uuid;
use warp::{Reply, reply::json};
use serde::{Serialize, Deserialize};
use futures::{SinkExt, StreamExt};
use futures::channel::mpsc;
use warp::ws::WebSocket;

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

// Always returns 200
pub async fn health_handler() -> Result<impl Reply> {
    let event = Event::Hide{id: 1};

    match serde_json::to_string(&event) {
        Ok(b) => { Ok(b) }
        Err(e) => { Ok(e.to_string()) }
    }
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

// Turns HTTP request into a websocket
pub async fn ws_handler(ws: warp::ws::Ws, id: String, clients: Clients) -> Result<impl Reply> {
    let client = clients.read().await.get(&id).cloned();
    match client {
        Some(c) => Ok(ws.on_upgrade(move |socket| client_connection(socket, id, clients, c))),
        None => Err(warp::reject::not_found()),
    }
}

// Handles the connection to the websocket
pub async fn client_connection(ws: WebSocket, id: String, clients: Clients, mut client: Client) {
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
        // Read message
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("error receiving ws message for id: {}): {}", id.clone(), e);
                break;
            }
        };

        // If message is string
        if let Ok(str) = msg.to_str() {
            // Parse message as event
            if let Ok(event) = serde_json::from_str::<Event>(&str) {
                // Send message to all clients
                for (cid, client) in clients.write().await.iter_mut() {
                    // Don't send the event to yourself
                    if id == *cid {
                        continue;
                    }

                    // Acutally send event
                    if let Some(sender) = &mut client.sender {
                        let _ = sender.send(Ok(msg.clone())).await;
                    }
                }
            }
        }
    }

    clients.write().await.remove(&id);
    println!("{} disconnected", id);
}
