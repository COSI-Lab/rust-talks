use diesel::{QueryDsl, RunQueryDsl, SqliteConnection, r2d2::{ConnectionManager, PooledConnection}};

use crate::{error::AppError, model::{CreateTalk, Talk}};
use crate::diesel::ExpressionMethods;

pub type PooledSqlite = PooledConnection<ConnectionManager<SqliteConnection>>;

pub struct DBManager {
    connection: PooledSqlite,
}

impl DBManager {
    pub fn new(connection: PooledSqlite) -> DBManager {
        DBManager { connection }
    }

    pub fn create_talk(&self, talk: CreateTalk) -> Result<usize, AppError> {
        use super::schema::talks;

        diesel::insert_into(talks::table) 
            .values(&talk)
            .execute(&self.connection)
            .map_err(|err| AppError::from_diesel_err(err, "while creating talk"))
    }

    pub fn list_visible_talks(&self) -> Result<Vec<Talk>, AppError> {
        use super::schema::talks::dsl::*;

        talks
            .filter(is_visible.eq(true))
            .load(&self.connection)
            .map_err(|err| {
                AppError::from_diesel_err(err, "listing visible talks")
            })
    }

    pub fn list_all_talks(&self) -> Result<Vec<Talk>, AppError> {
        use super::schema::talks::dsl::*;

        talks
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