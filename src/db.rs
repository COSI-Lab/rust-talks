use diesel::{Connection, QueryDsl, RunQueryDsl, SqliteConnection, Table, r2d2::{ConnectionManager, PooledConnection}, result::Error};

use crate::{error::AppError, model::{CreateTalk, Talk}};
use crate::diesel::ExpressionMethods;

pub type PooledSqlite = PooledConnection<ConnectionManager<SqliteConnection>>;

pub fn last_insert_rowid(c: &SqliteConnection) -> i32 {
    no_arg_sql_function!(last_insert_rowid, diesel::sql_types::Integer);
    match diesel::select(last_insert_rowid).first(c) {
        Ok(value) => value,
        _ => 0,
    }
}

pub struct DBManager {
    connection: PooledSqlite,
}

impl DBManager {
    pub fn new(connection: PooledSqlite) -> DBManager {
        DBManager { connection }
    }

    pub fn create_talk(&self, talk: CreateTalk) -> Result<i32, AppError> {
        use super::schema::talks;

        self.connection.transaction::<i32, _, _>(|| {
            let id = diesel::insert_into(talks::table) 
                .values(&talk)
                .execute(&self.connection)
                .map(|_| last_insert_rowid(&self.connection));

            match id {
                Ok(id) => { Ok(id) }
                Err(err) => { 
                    println!("{}", err);
                    Err(Error::RollbackTransaction)
                }
            }
        }).map_err(|err| { 
            println!("{}", err);
            AppError::from_diesel_err(err, "create talk")}
        )
    }

    pub fn list_visible_talks(&self) -> Result<Vec<Talk>, AppError> {
        use super::schema::talks::dsl::*;

        talks
            .filter(is_visible.eq(true))
            .order(talk_type)
            .load(&self.connection)
            .map_err(|err| {
                AppError::from_diesel_err(err, "listing visible talks")
            })
    }

    #[allow(dead_code)]
    pub fn list_all_talks(&self) -> Result<Vec<Talk>, AppError> {
        use super::schema::talks::dsl::*;

        talks
            .select(talks::all_columns())
            .load(&self.connection)
            .map_err(|err| {
                AppError::from_diesel_err(err, "listing all talks")
            })
    }

    pub fn hide_talk(&self, talk_id: i32) -> Result<usize, AppError> {
        use super::schema::talks::dsl::*;

        let talk = talks.find(talk_id);
        diesel::update(talk)
            .set(is_visible.eq(false))
            .execute(&self.connection)
            .map_err(|err| {
                AppError::from_diesel_err(err, &format!("unhiding talk {}", talk_id))
            })
    }

    #[allow(dead_code)]
    pub fn unhide_talk(&self, talk_id: i32) -> Result<usize, AppError> {
        use super::schema::talks::dsl::*;

        let talk = talks.find(talk_id);
        diesel::update(talk)
            .set(is_visible.eq(true))
            .execute(&self.connection)
            .map_err(|err| {
                AppError::from_diesel_err(err, &format!("unhiding talk {}", talk_id))
            })
    }

    #[allow(dead_code)]
    pub fn delete_talk(&self, talk_id: i32) -> Result<usize, AppError> {
        use super::schema::talks::dsl::*;

        let talk = talks.find(talk_id);
        diesel::delete(talk)
            .execute(&self.connection)
            .map_err(|err| {
                AppError::from_diesel_err(err, &format!("deleting talk {}", talk_id))
            })
    }
}