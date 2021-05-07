use std::{collections::HashMap, sync::Arc, time::Duration};

use futures::{StreamExt, channel::mpsc::{self, UnboundedSender}};
use tokio::sync::RwLock;
use warp::ws::{Message, WebSocket};

use crate::{db::DBManager, events::{EventRequest, process_event, send_events}};

// Clients type
pub type Clients = Arc<RwLock<HashMap<String, Client>>>;

pub fn create_clients() -> Clients {
    Arc::new(RwLock::new(HashMap::new()))
}

#[derive(Debug, Clone)]
pub struct Client {
    pub sender: Option<UnboundedSender<std::result::Result<Message, warp::Error>>>,
    pub authenticated: bool,
    pub second_chance: bool
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

// Forever run the garabage collector every minute
pub async fn garabage_collector(clients: Clients) {
    println!("Starting Garabage Collector");

    loop {
        std::thread::sleep(Duration::from_secs(60));
        let mut remove = Vec::new();

        {
            let mut writer = clients.write().await;
    
            for (k, v) in writer.iter_mut() {
                if v.sender.is_none() {
                    if v.second_chance {
                        v.second_chance = false;
                    } else {
                        remove.push(k.clone());
                    }
                }
            }
        }
    
        {
            let mut writer = clients.write().await;
            for k in remove {
                writer.remove(&k);
            }
        }

        if remove.len() > 0 {
            println!("cleaned {} clients", remove.len());
        }
    }
}