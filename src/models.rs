use chrono::NaiveDate;
use serde_derive::Serialize;

#[derive(Queryable, Clone, Debug, Serialize)]
pub struct BlogEntry {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub date: NaiveDate,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct BlogContext {
    pub blogentries: Vec<BlogEntry>,
}

use crate::schema::BlogEntries;

#[derive(Insertable)]
#[table_name = "BlogEntries"]
pub struct NewBlogEntry {
    pub title: String,
    pub description: String,
    pub date: NaiveDate,
    pub path: String,
}
