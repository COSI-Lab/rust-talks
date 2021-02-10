use std::{collections::HashMap, convert::Infallible, sync::Arc};
use events::{EventRequest, Talk};
use futures::channel::mpsc::{UnboundedSender, unbounded};
use tokio::sync::RwLock;
use warp::{Filter, Rejection, ws::Message};

mod handler;
mod events;

// Result type
type Result<T> = std::result::Result<T, Rejection>;
type Clients = Arc<RwLock<HashMap<String, Client>>>;
type EventQueue = Arc<RwLock<Queue>>;
type DB = Arc<RwLock<Vec<Talk>>>;

#[derive(Debug, Clone)]
pub struct Queue {
    pub queue: UnboundedSender<EventRequest>,
}

#[derive(Debug, Clone)]
pub struct Client {
    pub sender: Option<UnboundedSender<std::result::Result<Message, warp::Error>>>,
}

#[tokio::main]
async fn main() {
    // Current clients
    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

    // db of talks
    let db: DB = Arc::new(RwLock::new(Vec::new()));

    // Create MPSC event queue channel
    let (tx, rx) = unbounded::<EventRequest>();
    let queue: EventQueue = Arc::new(RwLock::new(Queue { queue: tx }));

    // Indicates whether the service is up
    let health_route = warp::path("health")
        .and_then(handler::health_handler);

    // Registers a new client for live updates
    let register = warp::path("register")
        .and(warp::post())
        .and(with_clients(clients.clone()))
        .and_then(handler::register_handler);

    // Gets talks route
    let talks = warp::path("talks")
        .and(with_db(db.clone()))
        .and_then(handler::visible_talks);

    // Websocket endpoint
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_clients(clients.clone()))
        .and(with_events(queue.clone()))
        .and_then(handler::ws_handler);

    // Combine all routes
    let routes = health_route
        .or(register)
        .or(talks)
        .or(ws_route)
        .with(warp::cors().allow_any_origin());
    
    // Create a new thread dedicated to processing incoming events
    tokio::task::spawn(events::process_events(rx, clients, db.clone()));

    // Serve the routes
    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}

// This is spooky code that allows handlers to access client object 
fn with_clients(clients: Clients) -> impl Filter<Extract = (Clients,), Error = Infallible> + Clone {
    warp::any().map(move || clients.clone())
}

// This is spooky code that allows handlers to access the event queue  
fn with_events(queue: EventQueue) -> impl Filter<Extract = (EventQueue,), Error = Infallible> + Clone {
    warp::any().map(move || queue.clone())
}

// This is spooky code that allows handlers to access the db of talks
fn with_db(db: DB) -> impl Filter<Extract = (DB,), Error = Infallible> + Clone {
    warp::any().map(move || db.clone())
}