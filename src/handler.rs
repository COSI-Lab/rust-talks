use uuid::Uuid;
use warp::{Reply, reply::json};
use serde::{Serialize};
use futures::{SinkExt, StreamExt};
use futures::channel::mpsc;
use warp::ws::WebSocket;

use crate::{Client, Clients, DB, EventQueue, Result, events::EventRequest};

#[derive(Serialize, Debug)]
pub struct RegisterResponse {
    url: String,
}

// Always returns 200
pub async fn health_handler() -> Result<impl Reply> {
    let x = EventRequest::Create { name: String::from("name"), talk_type: crate::events::TalkType::ForumTopic, desc: String::from("test") };
    match serde_json::to_string(&x) {
        Ok(s) => { Ok(s) }
        Err(_) => { Ok(String::from("not ok")) }
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
            if let Ok(event) = serde_json::from_str::<EventRequest>(&str) {
                let _ = queue.write().await.queue.send(event).await;
            }
        }
    }

    clients.write().await.remove(&id);
    println!("{} disconnected", id);
}
