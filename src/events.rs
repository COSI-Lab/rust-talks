use futures::{SinkExt, StreamExt, channel::mpsc::UnboundedReceiver};
use serde::{Serialize, Deserialize};
use warp::ws::Message;

use crate::Clients;

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

pub async fn process_events(mut rx: UnboundedReceiver<(Event, Message)>, clients: Clients) {
    // Process events forever
    while let Some((event, msg)) = rx.next().await {
        // Send message to all clients
        for (_, client) in clients.write().await.iter_mut() {
            // send event
            if let Some(sender) = &mut client.sender {
                let _ = sender.send(Ok(msg.clone())).await;
            }
        }
    }
}