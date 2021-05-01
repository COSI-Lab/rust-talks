use futures::SinkExt;
use serde::{Serialize, Deserialize};
use warp::ws::Message;

use crate::{Clients, db::DBManager, model::{CreateTalk, TalkType}};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "event")]
pub enum EventRequest {
    Create { name: String, talk_type: TalkType, desc: String },
    Hide { id: i32 },
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "event")]
pub enum EventResponse {
    Show { id: i32, name: String, talk_type: TalkType, description: String },
    Hide { id: i32 },
    Authenticate { authenticated: bool },
    NOP,
}

pub async fn send_events(clients: Clients, event: EventResponse) {
    // Don't bother sending NOPs
    if event == EventResponse::NOP {
        return;
    }

    // Check if the client is asking if it's authed
    if let Ok(str) = serde_json::to_string(&event) { // Encode the response as a string
        // Wrap the string as a text message
        let msg = Message::text(&str);

        // Send the message to all clients
        for (_, client) in clients.write().await.iter_mut() {
            if let Some(sender) = &mut client.sender {
                let m = Ok(msg.clone());
                let _ = sender.send(m).await;
            }
        }
    }
}

// Process a request and return a response
pub fn process_event(event: EventRequest, db: &DBManager) -> EventResponse {
    match event {
        EventRequest::Create { name, talk_type, desc } => {
            // Add talk to the database            
            let talk: CreateTalk = CreateTalk { name: &name, talk_type: talk_type, description: &desc, is_visible: true };

            // Return data
            match db.create_talk(talk) {
                Ok(id) => {
                    EventResponse::Show { id, name, talk_type, description: desc }
                }
                Err(_) => {
                    EventResponse::NOP
                }
            }
        }
        EventRequest::Hide { id } => {
            // Update the talk in the database
            let res = db.hide_talk(id);

            match res {
                Ok(_) => {
                    EventResponse::Hide { id }
                }
                Err(_) => {
                    EventResponse::NOP
                }
            }
        }
    }
}
