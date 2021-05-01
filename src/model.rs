use serde::{Deserialize, Serialize};

use crate::schema::talks;

use diesel::{backend::Backend, serialize::{ToSql, Output}, sql_types::Integer};
use std::{fmt::Display, io::Write};
use diesel::{serialize, deserialize};
use diesel::deserialize::FromSql;

#[derive(Deserialize, Debug, Copy, Clone, PartialEq, AsExpression, FromSqlRow)]
#[sql_type = "Integer"]
pub enum TalkType {
    ForumTopic,
    LightningTalk,
    ProjectUpdate,
	Announcement,
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
            1 => Ok(TalkType::LightningTalk),
            2 => Ok(TalkType::ProjectUpdate),
            3 => Ok(TalkType::Announcement),
            4 => Ok(TalkType::AfterMeetingSlot),
            int => Err(format!("Invalid TalkType {}", int).into()),
        }
    }
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
pub struct CreateTalk<'a> {
    pub name: &'a String,
    pub talk_type: TalkType,
    pub description: &'a String,
    pub is_visible: bool
}
