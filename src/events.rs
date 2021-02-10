use std::{fs::OpenOptions, io::Write};

use futures::{SinkExt, StreamExt, channel::mpsc::UnboundedReceiver};
use serde::{Serialize, Deserialize};
use warp::ws::Message;

use crate::{Clients, DB};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "event")]
pub enum EventRequest {
    Create { name: String, talk_type: TalkType, desc: String },
    Hide { id: usize },
}

#[derive(Serialize, Debug)]
#[serde(tag = "event")]
pub enum EventResponse {
    Show { id: usize, name: String, talk_type: TalkType, desc: String },
    Hide { id: usize },
}

#[derive(Serialize, Clone, Debug)]
pub struct Talk {
    pub id: usize,
    pub name: String,
    pub talk_type: TalkType,
    pub desc: String,
    pub is_visible: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TalkType {
    ForumTopic,
    LightningTalk,
    ProjectUpdate,
    Announcement,
    AfterMeetingSlot,
}

pub async fn process_events(mut rx: UnboundedReceiver<EventRequest>, clients: Clients, all: DB) {
    let mut events = OpenOptions::new().append(true).open("events.txt").expect("cannot open events file for appending");

    // Process events forever
    while let Some(event) = rx.next().await {
        let response = process_event(event, &all).await;

        // Encode the response as a string
        if let Ok(str) = serde_json::to_string(&response) {
            // Wrap the string as a text message
            let msg = Message::text(&str);

            // Send the message to all clients
            for (_, client) in clients.write().await.iter_mut() {
                if let Some(sender) = &mut client.sender {
                    let _ = sender.send(Ok(msg.clone())).await;
                }
            }

            // Writes the msg to the events file
            loop {
                if let Ok(_) = events.write_all(str.as_bytes()) {
                    break;
                }
            }
        }
    }
}

pub async fn process_event(event: EventRequest, all: &DB) -> EventResponse {
    match event {
        EventRequest::Create { name, talk_type, desc } => {
            // Get lock            
            let mut locked = all.write().await;
            let id = locked.len();
            let talk = Talk { id, name: name.clone(), talk_type: talk_type.clone(), desc: desc.clone(), is_visible: true };
            locked.push(talk);

            EventResponse::Show { id, name, talk_type, desc }
        }
        EventRequest::Hide { id } => {
            if let Some(mut talk) = all.write().await.get_mut(id) {
                talk.is_visible = false;
            }

            EventResponse::Hide { id: id }
        }
    }
}
