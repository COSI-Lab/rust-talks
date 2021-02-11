use std::{collections::HashMap, convert::Infallible, sync::Arc};
use events::{EventRequest, Talk};
use futures::channel::mpsc::{UnboundedSender, unbounded};
use tokio::sync::RwLock;
use warp::{Filter, Rejection, ws::Message};

mod handler;
mod events;

// Constants
const PORT: u16 = 3001;
const HOST: &str = "talks.cosi.clarkson.edu/";

// Result type
type Result<T> = std::result::Result<T, Rejection>;
type Clients = Arc<RwLock<HashMap<String, Client>>>;
type EventQueue = Arc<RwLock<Queue>>;
type DB = Arc<RwLock<Vec<Talk>>>;

#[derive(Debug, Clone)]
pub struct Queue {
    pub queue: UnboundedSender<(EventRequest, String)>,
}

#[derive(Debug, Clone)]
pub struct Client {
    pub sender: Option<UnboundedSender<std::result::Result<Message, warp::Error>>>,
    pub authenticated: bool
}

#[tokio::main]
async fn main() {
    // Current clients
    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

    // Create db
    println!("Generating in memory database...");
    let db = events::create_db().await;
    println!("Loaded {} talks", db.read().await.len());

    // Create MPSC event queue channel
    let (tx, rx) = unbounded::<(EventRequest, String)>();
    let queue: EventQueue = Arc::new(RwLock::new(Queue { queue: tx }));

    // Index.html welcome route
    let welcome_route = warp::path::end()
        .and(with_db(db.clone()))
        .and_then(handler::welcome_handler);

    // Indicates whether the service is up
    let health_route = warp::path("health")
        .and_then(handler::health_handler);

    // Registers a new client for live updates
    let register = warp::path("register")
        .and(warp::addr::remote())
        .and(with_clients(clients.clone()))
        .and_then(handler::register_handler);

    let authenticate = warp::path("authenticate")
        .and(warp::body::json())
        .and(with_clients(clients.clone()))
        .and_then(handler::authenticate);

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

    // Host static files in ./static
    let static_files = warp::path("static")
        .and(warp::fs::dir("static"));

    // Combine all routes
    let routes = welcome_route
        .or(health_route)
        .or(register)
        .or(authenticate)
        .or(talks)
        .or(ws_route)
        .or(static_files)
        .with(warp::cors().allow_any_origin());
    
    // Create a new thread dedicated to processing incoming events
    tokio::task::spawn(events::process_events(rx, clients, db.clone()));

    // Serve the routes
    println!("Serving on port {}...", PORT);
    warp::serve(routes).run(([127, 0, 0, 1], PORT)).await;
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