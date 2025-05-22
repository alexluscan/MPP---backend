use diesel::prelude::*;

diesel::table! {
    categories (id) {
        id -> Int4,
        name -> Varchar,
        description -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    products (id) {
        id -> Int4,
        name -> Varchar,
        price -> Float8,
        description -> Text,
        image -> Varchar,
        video -> Nullable<Varchar>,
        category_id -> Int4,
        user_id -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(products -> categories (category_id));
diesel::joinable!(products -> users (user_id));

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Varchar,
        password -> Varchar,
        role -> Varchar,
    }
}

diesel::table! {
    logs (id) {
        id -> Int4,
        user_id -> Int4,
        action -> Varchar,
        entity -> Varchar,
        entity_id -> Nullable<Int4>,
        timestamp -> Timestamp,
    }
}

diesel::table! {
    monitored_users (user_id) {
        user_id -> Int4,
        username -> Varchar,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    categories,
    products,
    users,
    logs,
    monitored_users,
); 