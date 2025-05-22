use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use diesel::prelude::*;
use crate::db::schema::{categories, products, users, logs, monitored_users};

#[derive(Queryable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = categories)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = categories)]
pub struct NewCategory {
    pub name: String,
    pub description: String,
}

#[derive(Queryable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = products)]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub price: f64,
    pub description: String,
    pub image: String,
    pub video: Option<String>,
    pub category_id: i32,
    pub user_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = products)]
pub struct NewProduct {
    pub name: String,
    pub price: f64,
    pub description: String,
    pub image: String,
    pub video: Option<String>,
    pub category_id: i32,
    pub user_id: i32,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = products)]
pub struct UpdateProduct {
    pub name: Option<String>,
    pub price: Option<f64>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub video: Option<String>,
    pub category_id: Option<i32>,
    pub user_id: Option<i32>,
}

#[derive(AsChangeset, Deserialize)]
#[diesel(table_name = categories)]
pub struct UpdateCategory {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Queryable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password: String, // plain text (for demo only)
    pub role: String, // 'User' or 'Admin'
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub role: String,
}

#[derive(Queryable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = logs)]
pub struct Log {
    pub id: i32,
    pub user_id: i32,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<i32>,
    pub timestamp: NaiveDateTime,
}

#[derive(Insertable, Deserialize)]
#[diesel(table_name = logs)]
pub struct NewLog {
    pub user_id: i32,
    pub action: String,
    pub entity: String,
    pub entity_id: Option<i32>,
    pub timestamp: NaiveDateTime,
}

#[derive(Queryable, Serialize, Debug, Clone)]
#[diesel(table_name = monitored_users)]
pub struct MonitoredUser {
    pub user_id: i32,
    pub username: String,
} 