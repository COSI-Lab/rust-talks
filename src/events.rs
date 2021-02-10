use std::{fmt::Display, fs::{File, OpenOptions}, io::{BufRead, BufReader, Write}, sync::Arc};

use futures::{SinkExt, StreamExt, channel::mpsc::UnboundedReceiver};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use warp::ws::Message;

use crate::{Clients, DB};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "event")]
pub enum EventRequest {
    Create { name: String, talk_type: TalkType, desc: String },
    Hide { id: usize },
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TalkType {
    ForumTopic,
    LightningTalk,
    ProjectUpdate,
    Announcement,
    AfterMeetingSlot,
}

impl Display for TalkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            TalkType::ForumTopic => { f.write_str("Forum Topic") }
            TalkType::LightningTalk => { f.write_str("Lightning Talk") }
            TalkType::ProjectUpdate => { f.write_str("Project Update") }
            TalkType::Announcement => { f.write_str("Announcement") }
            TalkType::AfterMeetingSlot => { f.write_str("After Meeting Slot") }
        }
    }
}

pub async fn process_events(mut rx: UnboundedReceiver<(EventRequest, String)>, clients: Clients, all: DB) {
    let mut events = OpenOptions::new().append(true).open("events.txt").expect("cannot open events file for appending");

    // Process events forever
    while let Some((event, msg)) = rx.next().await {
        let response = process_event(event.clone(), &all).await;

        // Encode the response as a string
        if let Ok(str) = serde_json::to_string(&response) {
            // Writes the msg to the events file
            loop {
                if let Ok(_) = events.write_all(msg.as_bytes()) {
                    break;
                }
            }

            // Wrap the string as a text message
            let msg = Message::text(&str);

            // Send the message to all clients
            for (_, client) in clients.write().await.iter_mut() {
                if let Some(sender) = &mut client.sender {
                    let _ = sender.send(Ok(msg.clone())).await;
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
            locked.sort_by_cached_key(|talk| talk.talk_type.clone());

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

// Creates the DB object by running all events from events.txt
pub async fn create_db() -> DB {
    // Create the DB
    let db: DB = Arc::new(RwLock::new(Vec::new()));

    // Open the file in read-only mode 
    let file = File::open("events.txt").unwrap();
    let reader = BufReader::new(file);

    // Read the file line by line using the lines() iterator from std::io::BufRead.
    for line in reader.lines() {
        let event = serde_json::from_str::<EventRequest>(&line.unwrap()); // Ignore errors.

        match event {
            Ok(event) => { process_event(event, &db).await; }
            Err(err) => { println!("Error could not process event {}", err) }
        }
    }

    return db;
}