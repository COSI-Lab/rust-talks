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
    Authenticate { authenticated: bool }
}

#[derive(Serialize, Clone, Debug)]
pub struct Talk {
    pub id: usize,
    pub name: String,
    pub talk_type: TalkType,
    pub desc: String,
    pub is_visible: bool,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TalkType {
    ForumTopic,
    LightningTalk,
    ProjectUpdate,
    Announcement,
    AfterMeetingSlot,
}

impl Serialize for TalkType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_str(match *self {
            TalkType::ForumTopic => "forum topic",
            TalkType::LightningTalk => "lightning talk",
            TalkType::ProjectUpdate => "project update",
            TalkType::Announcement => "announcement",
            TalkType::AfterMeetingSlot => "after meeting slot"
        })
    }
}

impl Display for TalkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            TalkType::ForumTopic => { f.write_str("forum topic") }
            TalkType::LightningTalk => { f.write_str("lightning talk") }
            TalkType::ProjectUpdate => { f.write_str("project update") }
            TalkType::Announcement => { f.write_str("announcement") }
            TalkType::AfterMeetingSlot => { f.write_str("after meeting slot") }
        }
    }
}

pub async fn process_events(mut rx: UnboundedReceiver<(EventRequest, String)>, clients: Clients, all: DB) {
    let mut events = OpenOptions::new().append(true).open("events.txt").expect("cannot open events file for appending");

    // Process events forever
    while let Some((event, msg)) = rx.next().await {
        let response = process_event(event.clone(), &all).await;

        // Check if the client is asking if it's authed
        if let Ok(str) = serde_json::to_string(&response) { // Encode the response as a string
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

// Process a request and return a response
pub async fn process_event(event: EventRequest, db: &DB) -> EventResponse {
    match event {
        EventRequest::Create { name, talk_type, desc } => {
            // Get lock            
            let mut locked = db.write().await;
            
            // Add talk to the database
            let id = locked.len();
            let talk = Talk { id, name: name.clone(), talk_type: talk_type.clone(), desc: desc.clone(), is_visible: true };
            locked.push(talk);

            // Sort the database
            locked.sort_by_cached_key(|talk| talk.talk_type.clone());

            // Return data
            EventResponse::Show { id, name, talk_type, desc }
        }
        EventRequest::Hide { id } => {
            // This is technically O(n) but since we can assume any talk being hiden is relatively
            // "new" iterating in reverse can save a lot of clock cycles
            for mut talk in db.write().await.iter_mut().rev() {
                if talk.id == id {
                    talk.is_visible = false;
                    break;
                }
            };

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