#[macro_use]
extern crate diesel;

use std::env;
use db::DBManager;
use error::{AppError, ErrorType};
use warp::{Filter, reject};
use log::info;

pub mod schema;
pub mod model;
pub mod error;
pub mod db;

use diesel::{SqliteConnection, r2d2::{ConnectionManager, Pool}};

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

fn sqlite_pool(db_url: &str) -> SqlitePool {
    let manager = ConnectionManager::<SqliteConnection>::new(db_url);
    Pool::new(manager).expect("Postgres connection pool could not be created")
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

#[tokio::main]
async fn main() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let routes = warp::path!("hello").map(|| "Hello World!".to_string());

    info!("Starting server on port 3030...");

    // Start up the server...
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

