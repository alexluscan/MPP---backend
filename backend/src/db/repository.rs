use diesel::prelude::*;
use diesel::dsl::*;
use chrono::Utc;
use crate::db::connection::get_conn;
use crate::db::models::*;
use crate::db::schema::*;
use serde::Serialize;

pub struct ProductRepository;

impl ProductRepository {
    pub fn create(new_product: NewProduct) -> Result<Product, diesel::result::Error> {
        let conn = &mut get_conn();
        diesel::insert_into(products::table)
            .values(&new_product)
            .get_result(conn)
    }

    pub fn get_by_id(id: i32) -> Result<Product, diesel::result::Error> {
        let conn = &mut get_conn();
        products::table.find(id).first(conn)
    }

    pub fn update(id: i32, update_data: UpdateProduct) -> Result<Product, diesel::result::Error> {
        let conn = &mut get_conn();
        diesel::update(products::table.find(id))
            .set((
                update_data,
                products::updated_at.eq(Utc::now().naive_utc()),
            ))
            .get_result(conn)
    }

    pub fn delete(id: i32) -> Result<usize, diesel::result::Error> {
        let conn = &mut get_conn();
        diesel::delete(products::table.find(id)).execute(conn)
    }

    pub fn get_all(
        category_id: Option<i32>,
        min_price: Option<f64>,
        max_price: Option<f64>,
        search_term: Option<String>,
        sort_by: Option<String>,
        sort_order: Option<String>,
    ) -> Result<Vec<Product>, diesel::result::Error> {
        let conn = &mut get_conn();
        let mut query = products::table.into_boxed();

        if let Some(cat_id) = category_id {
            query = query.filter(products::category_id.eq(cat_id));
        }

        if let Some(min) = min_price {
            query = query.filter(products::price.ge(min));
        }

        if let Some(max) = max_price {
            query = query.filter(products::price.le(max));
        }

        if let Some(term) = search_term {
            let search_pattern = format!("%{}%", term);
            let pattern = search_pattern.clone();
            query = query.filter(
                products::name.ilike(pattern)
                    .or(products::description.ilike(search_pattern))
            );
        }

        if let Some(sort) = sort_by {
            let order = sort_order.unwrap_or_else(|| "asc".to_string());
            query = match (sort.as_str(), order.as_str()) {
                ("name", "asc") => query.order(products::name.asc()),
                ("name", "desc") => query.order(products::name.desc()),
                ("price", "asc") => query.order(products::price.asc()),
                ("price", "desc") => query.order(products::price.desc()),
                ("created_at", "asc") => query.order(products::created_at.asc()),
                ("created_at", "desc") => query.order(products::created_at.desc()),
                _ => query.order(products::id.asc()),
            };
        } else {
            query = query.order(products::id.asc());
        }

        query.load(conn)
    }
}

pub struct CategoryRepository;

impl CategoryRepository {
    pub fn create(new_category: NewCategory) -> Result<Category, diesel::result::Error> {
        let conn = &mut get_conn();
        diesel::insert_into(categories::table)
            .values(&new_category)
            .get_result(conn)
    }

    pub fn get_by_id(id: i32) -> Result<Category, diesel::result::Error> {
        let conn = &mut get_conn();
        categories::table.find(id).first(conn)
    }

    pub fn update(id: i32, update_data: UpdateCategory) -> Result<Category, diesel::result::Error> {
        let conn = &mut get_conn();
        diesel::update(categories::table.find(id))
            .set((
                update_data,
                categories::updated_at.eq(Utc::now().naive_utc()),
            ))
            .get_result(conn)
    }

    pub fn delete(id: i32) -> Result<usize, diesel::result::Error> {
        let conn = &mut get_conn();
        diesel::delete(categories::table.find(id)).execute(conn)
    }

    pub fn get_all() -> Result<Vec<Category>, diesel::result::Error> {
        let conn = &mut get_conn();
        categories::table.order(categories::name.asc()).load(conn)
    }

    pub fn get_products_by_category(category_id: i32) -> Result<Vec<Product>, diesel::result::Error> {
        let conn = &mut get_conn();
        products::table
            .filter(products::category_id.eq(category_id))
            .order(products::name.asc())
            .load(conn)
    }
}

pub fn create_product(conn: &mut PgConnection, new_product: NewProduct) -> QueryResult<Product> {
    diesel::insert_into(products::table)
        .values(new_product)
        .get_result(conn)
}

pub fn get_product(conn: &mut PgConnection, id: i32) -> QueryResult<Product> {
    products::table.find(id).first(conn)
}

pub fn update_product(conn: &mut PgConnection, id: i32, product: UpdateProduct) -> QueryResult<Product> {
    diesel::update(products::table.find(id))
        .set((
            product,
            products::updated_at.eq(Utc::now().naive_utc()),
        ))
        .get_result(conn)
}

pub fn delete_product(conn: &mut PgConnection, id: i32) -> QueryResult<usize> {
    diesel::delete(products::table.find(id)).execute(conn)
}

pub fn create_category(conn: &mut PgConnection, new_category: NewCategory) -> QueryResult<Category> {
    diesel::insert_into(categories::table)
        .values(new_category)
        .get_result(conn)
}

pub fn get_category(conn: &mut PgConnection, id: i32) -> QueryResult<Category> {
    categories::table.find(id).first(conn)
}

pub fn update_category(conn: &mut PgConnection, id: i32, category: UpdateCategory) -> QueryResult<Category> {
    diesel::update(categories::table.find(id))
        .set((
            category,
            categories::updated_at.eq(Utc::now().naive_utc()),
        ))
        .get_result(conn)
}

pub fn delete_category(conn: &mut PgConnection, id: i32) -> QueryResult<usize> {
    diesel::delete(categories::table.find(id)).execute(conn)
}

pub fn get_all_categories(conn: &mut PgConnection) -> QueryResult<Vec<Category>> {
    categories::table.order(categories::name.asc()).load(conn)
}

pub fn get_all_products(conn: &mut PgConnection, user_id: i32) -> QueryResult<Vec<Product>> {
    products::table
        .filter(products::user_id.eq(user_id))
        .load(conn)
}

pub fn get_products_by_category(conn: &mut PgConnection, category_id: i32, user_id: i32) -> QueryResult<Vec<Product>> {
    products::table
        .filter(products::category_id.eq(category_id))
        .filter(products::user_id.eq(user_id))
        .load(conn)
}

#[derive(Queryable, Serialize)]
pub struct ProductWithCategory {
    pub id: i32,
    pub name: String,
    pub price: f64,
    pub description: String,
    pub image: String,
    pub video: Option<String>,
    pub category_id: i32,
    pub category_name: String,
    pub user_id: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

pub fn get_all_products_with_category(conn: &mut PgConnection, user_id_val: i32) -> QueryResult<Vec<ProductWithCategory>> {
    use crate::db::schema::products;
    use crate::db::schema::categories::dsl::{categories, name as cat_name, id as cat_id};
    
    products::table
        .inner_join(categories.on(products::category_id.eq(cat_id)))
        .filter(products::user_id.eq(user_id_val))
        .select((
            products::id,
            products::name,
            products::price,
            products::description,
            products::image,
            products::video,
            products::category_id,
            cat_name,
            products::user_id,
            products::created_at,
            products::updated_at,
        ))
        .load::<ProductWithCategory>(conn)
}

pub fn get_all_products_with_category_all_users(conn: &mut PgConnection) -> QueryResult<Vec<ProductWithCategory>> {
    use crate::db::schema::products;
    use crate::db::schema::categories::dsl::{categories, name as cat_name, id as cat_id};
    products::table
        .inner_join(categories.on(products::category_id.eq(cat_id)))
        .select((
            products::id,
            products::name,
            products::price,
            products::description,
            products::image,
            products::video,
            products::category_id,
            cat_name,
            products::user_id,
            products::created_at,
            products::updated_at,
        ))
        .load::<ProductWithCategory>(conn)
}

pub fn get_monitored_users(conn: &mut PgConnection) -> QueryResult<Vec<MonitoredUser>> {
    use crate::db::schema::monitored_users::dsl::*;
    monitored_users.load::<MonitoredUser>(conn)
}

pub fn add_monitored_user(conn: &mut PgConnection, user_id_val: i32, username_val: &str) -> QueryResult<usize> {
    use crate::db::schema::monitored_users;
    diesel::insert_into(monitored_users::table)
        .values((monitored_users::user_id.eq(user_id_val), monitored_users::username.eq(username_val)))
        .on_conflict(monitored_users::user_id)
        .do_nothing()
        .execute(conn)
}

pub fn clear_monitored_users(conn: &mut PgConnection) -> QueryResult<usize> {
    use crate::db::schema::monitored_users::dsl::*;
    diesel::delete(monitored_users).execute(conn)
} 