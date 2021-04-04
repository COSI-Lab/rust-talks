use serde::{Deserialize, Serialize};

use crate::schema::talks;

use diesel::{backend::Backend, serialize::{ToSql, Output}, sql_types::Integer};
use std::io::Write;
use diesel::{serialize, deserialize};
use diesel::deserialize::FromSql;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, AsExpression, FromSqlRow)]
#[sql_type = "Integer"]
pub enum TalkType {
    ForumTopic,
    LightingTalk,
    ProjectUpdates,
	Announcements,
    AfterMeetingSlot
}

// Converts TalkType enum to sql interger
impl<DB> ToSql<Integer, DB> for TalkType
where
    DB: Backend,
    i32: ToSql<Integer, DB>,
{
    fn to_sql<W: Write>(&self, out: &mut Output<W, DB>) -> serialize::Result {
        (*self as i32).to_sql(out)
    }
}

impl<DB> FromSql<Integer, DB> for TalkType
where
    DB: Backend,
    i32: FromSql<Integer, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        match i32::from_sql(bytes)? {
            0 => Ok(TalkType::ForumTopic),
            1 => Ok(TalkType::LightingTalk),
            2 => Ok(TalkType::ProjectUpdates),
            3 => Ok(TalkType::Announcements),
            4 => Ok(TalkType::AfterMeetingSlot),
            int => Err(format!("Invalid TalkType {}", int).into()),
        }
    }
}

#[derive(Serialize, Debug, Clone, Queryable)]
pub struct Talk {
    pub id: i32,
    pub name: String,
    pub talk_type: TalkType,
    pub description: String,
    pub is_visible: bool
}

// Struct for creating Book
#[derive(Debug, Clone, Insertable)]
#[table_name = "talks"]
pub struct CreateTalk {
    pub name: String,
    pub talk_type: TalkType,
    pub description: String,
    pub is_visible: bool
}
