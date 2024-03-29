#[macro_use]
extern crate diesel;

use std::{convert::Infallible, env};
use db::DBManager;
use diesel::{SqliteConnection, r2d2::{ConnectionManager, Pool}};
use error::{AppError, ErrorType};
use warp::{Filter, hyper::Uri, reject};

use crate::client::{Clients, create_clients, garabage_collector};

mod api;
mod events;
mod db;
mod error;
mod model;
mod client;
pub mod schema;

#[tokio::main]
async fn main() {
    // Current clients
    let clients: Clients = create_clients();
    
    // Create db
    let database_url = {
        let url = env::var_os("DATABASE_URL");
        match url {
            Some(url) => {
                url.into_string().unwrap()
            }
            None => {
                String::from("talks.db")
            }
        }
    };

    // Get the debug status
    let is_debug = {
        let var = env::var_os("DEBUG");
        match var {
            Some(_) => {
                true
            }
            None => {
                false
            }
        }
    };

    let pool = sqlite_pool(&database_url);

    // index welcome route
    let welcome_route = warp::path::end()
        .and(with_db_access_manager(pool.clone()))
        .and_then(api::welcome_handler);

    // show all talks in order by id (chronological order)
    let all_route = warp::path("all")
        .and(with_db_access_manager(pool.clone()))
        .and_then(api::all_talks_handler);

    // Indicates whether the service is up
    let health_route = warp::path("health")
        .and(with_clients(clients.clone()))
        .and_then(api::health_handler);

    // Registers a new client for live updates
    let register = warp::path("register")
        .and(warp::header::optional("x-forwarded-for"))
        .and(with_clients(clients.clone()))
        .and_then(api::register_handler);

    let authenticate = warp::path("authenticate")
        .and(warp::body::json())
        .and(with_clients(clients.clone()))
        .and_then(api::authenticate);

    // Gets talks route
    let talks = warp::path("talks")
        .and(with_db_access_manager(pool.clone()))
        .and_then(api::visible_talks);

    // Websocket endpoint
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_clients(clients.clone()))
        .and(with_db_access_manager(pool.clone()))
        .and_then(api::ws_handler);

    // Host static files in ./static
    let static_files = warp::path("static")
        .and(warp::fs::dir("static"));

    // start garabage collector
    tokio::spawn(garabage_collector(clients));

    // Serve the routes
    let port = std::option_env!("VIRTUAL_PORT").unwrap_or("8000").parse::<u16>().unwrap();

    let valid = if !is_debug {
        warp::host::exact("talks.cosi.clarkson.edu")
        .or(warp::host::exact("talks.cslabs.clarkson.edu"))
        .unify().boxed()
    } else {
        warp::any().boxed()
    };

    let routes = valid
    .and(
        welcome_route
        .or(health_route)
        .or(all_route)
        .or(register)
        .or(authenticate)
        .or(talks)
        .or(ws_route)
        .or(static_files)
    );

    if is_debug {
        println!("Serving debug on port {}...", port);
        warp::serve(routes).run(([0, 0, 0, 0], port)).await;    
    } else {
        println!("Serving on port {}...", port);
        warp::serve(routes.or(
            warp::any()
            .map(move || warp::redirect::temporary(Uri::from_static("https://talks.cosi.clarkson.edu")))
        )).run(([0, 0, 0, 0], port)).await;    
    }
}

// This is spooky code that allows handlers to access client object 
fn with_clients(clients: Clients) -> impl Filter<Extract = (Clients,), Error = Infallible> + Clone {
    warp::any().map(move || clients.clone())
}

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

fn sqlite_pool(db_url: &str) -> SqlitePool {
    let manager = ConnectionManager::<SqliteConnection>::new(db_url);
    Pool::new(manager).expect("Sqlite connection pool could not be created")
}

fn with_db_access_manager(pool: SqlitePool) -> impl Filter<Extract = (DBManager,), Error = warp::Rejection> + Clone {
    warp::any()
        .map(move || pool.clone())
        .and_then(|pool: SqlitePool| async move {  match pool.get() {
            Ok(conn) => Ok(DBManager::new(conn)),
            Err(err) => Err(reject::custom(
                AppError::new(format!("Error getting connection from pool: {}", err.to_string()).as_str(), ErrorType::Internal))
            ),
        }})
}
