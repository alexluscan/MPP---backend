use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder, FromRequest, dev::Payload, Error as ActixError, HttpRequest};
use actix_files::Files;
use futures::StreamExt;
use serde::Deserialize;
use std::sync::Mutex;
use rand::Rng;
use models::{Product, CreateProductRequest, ProductQuery, CreateCategoryRequest};
use actix_web_actors::ws;
use actix::prelude::*;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use serde_json::json;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use chrono::Utc;
use crate::db::models::{Category, NewProduct, NewCategory, UpdateProduct, UpdateCategory, User, NewUser, Log, NewLog};
use crate::db::repository;
use actix_web::web::Bytes;
use serde_json::Value;
use diesel::insert_into;
use chrono::NaiveDateTime;
use actix_web::middleware::Logger;
use futures::future::{ready, Ready};
use std::collections::HashMap;
use actix_web::web::Query;

mod models;
mod db;
mod mock_data;

// Global state to store products
pub struct AppState {
    pool: DbPool,
}

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

fn filter_and_sort_products(products: &[Product], query: &ProductQuery) -> Vec<Product> {
    let mut filtered = products.to_vec();

    // Apply filters
    if let Some(category_id) = &query.category_id {
        filtered.retain(|p| p.category_id == *category_id);
    }

    if let Some(min_price) = query.min_price {
        filtered.retain(|p| (p.price * 100.0).round() >= (min_price * 100.0).round());
    }

    if let Some(max_price) = query.max_price {
        filtered.retain(|p| (p.price * 100.0).round() <= (max_price * 100.0).round());
    }

    if let Some(search_term) = &query.search_term {
        let search_term = search_term.to_lowercase();
        filtered.retain(|p| {
            p.name.to_lowercase().contains(&search_term) ||
            p.description.to_lowercase().contains(&search_term)
        });
    }

    // Apply sorting
    if let Some(sort_by) = &query.sort_by {
        let sort_order = query.sort_order.as_deref().unwrap_or("asc");
        filtered.sort_by(|a, b| {
            let comparison = match sort_by.as_str() {
                "name" => a.name.cmp(&b.name),
                "price" => a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal),
                _ => std::cmp::Ordering::Equal,
            };
            if sort_order == "desc" {
                comparison.reverse()
            } else {
                comparison
            }
        });
    }

    filtered
}

// Validation function for product data
fn validate_product(product: &CreateProductRequest) -> Result<(), String> {
    if product.name.trim().is_empty() {
        return Err("Product name cannot be empty".to_string());
    }
    if product.price <= 0.0 {
        return Err("Product price must be greater than 0".to_string());
    }
    if product.category_id <= 0 {
        return Err("Invalid category ID".to_string());
    }
    if product.description.trim().is_empty() {
        return Err("Product description cannot be empty".to_string());
    }
    if product.image.trim().is_empty() {
        return Err("Product image URL cannot be empty".to_string());
    }
    Ok(())
}

#[derive(Deserialize, Clone)]
struct AuthUser {
    user_id: i32,
}

impl FromRequest for AuthUser {
    type Error = ActixError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        // Expect header: Authorization: Bearer <user_id>
        if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    if let Ok(user_id) = token.trim().parse::<i32>() {
                        return ready(Ok(AuthUser { user_id }));
                    }
                }
            }
        }
        // If missing or invalid, return error
        ready(Err(actix_web::error::ErrorUnauthorized("Missing or invalid Authorization header")))
    }
}

async fn get_products(
    data: web::Data<AppState>,
    query: web::Query<ProductQuery>,
) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    let mut products = repository::get_all_products_with_category_all_users(conn).unwrap_or_default();

    // Apply category filter
    if let Some(category_id) = query.category_id {
        products.retain(|p| p.category_id == category_id);
    }

    // Apply price filters
    if let Some(min_price) = query.min_price {
        products.retain(|p| p.price >= min_price);
    }
    if let Some(max_price) = query.max_price {
        products.retain(|p| p.price <= max_price);
    }

    // Apply search term
    if let Some(search_term) = &query.search_term {
        let search_term = search_term.to_lowercase();
        products.retain(|p| {
            p.name.to_lowercase().contains(&search_term) ||
            p.description.to_lowercase().contains(&search_term)
        });
    }

    // Apply sorting
    if let Some(sort_by) = &query.sort_by {
        let sort_order = query.sort_order.as_deref().unwrap_or("asc");
        products.sort_by(|a, b| {
            let cmp = match sort_by.as_str() {
                "name" => a.name.cmp(&b.name),
                "price" => a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal),
                _ => std::cmp::Ordering::Equal,
            };
            if sort_order == "desc" {
                cmp.reverse()
            } else {
                cmp
            }
        });
    }

    HttpResponse::Ok().json(products)
}

async fn get_product(data: web::Data<AppState>, id: web::Path<i32>) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    match repository::get_product(conn, id.into_inner()) {
        Ok(product) => HttpResponse::Ok().json(product),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    password: String,
    role: Option<String>, // Optional, default to User
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

// Registration endpoint
async fn register(
    data: web::Data<AppState>,
    req: web::Json<RegisterRequest>,
) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    // Check if username exists
    use crate::db::schema::users::dsl::*;
    if users.filter(username.eq(&req.username)).first::<User>(conn).is_ok() {
        return HttpResponse::BadRequest().json(json!({"message": "Username already exists"}));
    }
    let new_user = NewUser {
        username: req.username.clone(),
        password: req.password.clone(), // plain text for demo
        role: req.role.clone().unwrap_or_else(|| "User".to_string()),
    };
    match insert_into(users).values(&new_user).get_result::<User>(conn) {
        Ok(user) => HttpResponse::Ok().json(json!({"id": user.id, "username": user.username, "role": user.role})),
        Err(_) => HttpResponse::InternalServerError().json(json!({"message": "Failed to register user"})),
    }
}

// Login endpoint
async fn login(
    data: web::Data<AppState>,
    req: web::Json<LoginRequest>,
) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    use crate::db::schema::users::dsl::*;
    match users.filter(username.eq(&req.username)).first::<User>(conn) {
        Ok(user) => {
            if user.password == req.password {
                HttpResponse::Ok().json(json!({"id": user.id, "username": user.username, "role": user.role}))
            } else {
                HttpResponse::Unauthorized().json(json!({"message": "Invalid password"}))
            }
        }
        Err(_) => HttpResponse::Unauthorized().json(json!({"message": "User not found"})),
    }
}

// Helper to log actions
fn log_action(conn: &mut PgConnection, user_id_val: i32, action_val: &str, entity_val: &str, entity_id_val: Option<i32>) {
    use crate::db::schema::logs::dsl::*;
    use crate::db::schema::users::dsl as users_dsl;
    use crate::db::repository::add_monitored_user;
    let new_log = NewLog {
        user_id: user_id_val,
        action: action_val.to_string(),
        entity: entity_val.to_string(),
        entity_id: entity_id_val,
        timestamp: chrono::Utc::now().naive_utc(),
    };
    let _ = insert_into(logs).values(&new_log).execute(conn);
    // Check for suspicious activity: more than 10 logs in the last minute
    let one_min_ago = chrono::Utc::now().naive_utc() - chrono::Duration::minutes(1);
    let count: i64 = logs
        .filter(user_id.eq(user_id_val))
        .filter(timestamp.ge(one_min_ago))
        .count()
        .get_result(conn)
        .unwrap_or(0);
    if count > 10 {
        // Get username
        if let Ok(user) = users_dsl::users.find(user_id_val).first::<crate::db::models::User>(conn) {
            let _ = add_monitored_user(conn, user_id_val, &user.username);
        }
    }
}

// Example: log product creation in create_product
async fn create_product(
    data: web::Data<AppState>,
    product: web::Json<NewProduct>,
) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    let new_product = product.into_inner();
    match repository::create_product(conn, new_product) {
        Ok(product) => {
            log_action(conn, product.user_id, "CREATE", "product", Some(product.id));
            HttpResponse::Created().json(product)
        },
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn update_product(
    data: web::Data<AppState>,
    id: web::Path<i32>,
    product: web::Json<UpdateProduct>,
) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    let product_id = id.into_inner();
    let user_id = product.user_id.unwrap_or(0); // fallback if not provided
    match repository::update_product(conn, product_id, product.into_inner()) {
        Ok(product) => {
            log_action(conn, user_id, "UPDATE", "product", Some(product.id));
            HttpResponse::Ok().json(product)
        },
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn delete_product(
    data: web::Data<AppState>,
    id: web::Path<i32>,
) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    let product_id = id.into_inner();
    // Try to get the product before deleting to get user_id
    let user_id = match repository::get_product(conn, product_id) {
        Ok(product) => product.user_id,
        Err(_) => 0,
    };
    match repository::delete_product(conn, product_id) {
        Ok(_) => {
            log_action(conn, user_id, "DELETE", "product", Some(product_id));
            HttpResponse::NoContent().finish()
        },
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn get_categories(data: web::Data<AppState>) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    match repository::get_all_categories(conn) {
        Ok(categories) => HttpResponse::Ok().json(categories),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn get_category(data: web::Data<AppState>, id: web::Path<i32>) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    match repository::get_category(conn, id.into_inner()) {
        Ok(category) => HttpResponse::Ok().json(category),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

async fn create_category(data: web::Data<AppState>, category: web::Json<CreateCategoryRequest>) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    
    if category.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"message": "Category name cannot be empty"}));
    }

    let new_category = NewCategory {
        name: category.name.clone(),
        description: category.description.clone(),
    };

    match repository::create_category(conn, new_category) {
        Ok(category) => HttpResponse::Created().json(category),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

async fn update_category(
    data: web::Data<AppState>,
    id: web::Path<i32>,
    category: web::Json<CreateCategoryRequest>,
) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();

    if category.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"message": "Category name cannot be empty"}));
    }

    let update_category = UpdateCategory {
        name: Some(category.name.clone()),
        description: Some(category.description.clone()),
    };

    match repository::update_category(conn, id.into_inner(), update_category) {
        Ok(category) => HttpResponse::Ok().json(category),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

async fn delete_category(data: web::Data<AppState>, id: web::Path<i32>) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    match repository::delete_category(conn, id.into_inner()) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

// Add the generation status as app data
async fn start_server() -> std::io::Result<()> {
    println!("Initializing server...");
    let generation_status = Arc::new(AtomicBool::new(false));
    let app_state = web::Data::new(AppState {
        pool: r2d2::Pool::builder()
            .build(ConnectionManager::<PgConnection>::new("postgres://postgres:luscan@localhost:5432/postgres"))
            .expect("Failed to create pool"),
    });

    println!("Starting HTTP server on http://localhost:3001");
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .allowed_header("content-type")
            .allowed_header("authorization")
            .allowed_header("sec-websocket-key")
            .allowed_header("sec-websocket-protocol")
            .allowed_header("sec-websocket-version")
            .allowed_header("upgrade")
            .allowed_header("connection")
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(generation_status.clone()))
            .app_data(app_state.clone())
            .app_data(web::Data::new(web::PayloadConfig::new(100 * 1024 * 1024)))
            .service(Files::new("/videos", "videos").show_files_listing())
            .route("/api/get/products", web::get().to(get_products))
            .route("/api/post/products", web::post().to(create_product))
            .route("/api/get/products/{id}", web::get().to(get_product))
            .route("/api/patch/products/{id}", web::patch().to(update_product))
            .route("/api/delete/products/{id}", web::delete().to(delete_product))
            .route("/api/get/categories", web::get().to(get_categories))
            .route("/api/post/categories", web::post().to(create_category))
            .route("/api/get/categories/{id}", web::get().to(get_category))
            .route("/api/patch/categories/{id}", web::put().to(update_category))
            .route("/api/delete/categories/{id}", web::delete().to(delete_category))
            .route("/api/toggle-generation", web::post().to(toggle_generation))
            .route("/api/shutdown", web::get().to(shutdown_server))
            .route("/api/register", web::post().to(register))
            .route("/api/login", web::post().to(login))
            .route("/api/get/products/user/{user_id}", web::get().to(get_products_by_user_id))
            .route("/api/monitored-users", web::get().to(get_monitored_users_handler))
            .route("/api/stats/avg-price-per-category", web::get().to(avg_price_per_category_handler))
            .route("/api/stats/avg-price-inefficient", web::get().to(avg_price_inefficient_handler))
            .route("/api/stats/avg-price-per-category-inefficient", web::get().to(avg_price_per_category_inefficient_handler))
    })
    .bind("127.0.0.1:3001")?
    .run()
    .await
}

async fn toggle_generation(generation_status: web::Data<Arc<AtomicBool>>) -> impl Responder {
    let current = generation_status.load(Ordering::SeqCst);
    let new_status = !current;
    generation_status.store(new_status, Ordering::SeqCst);
    println!("Product generation toggled: {} -> {}", current, new_status);
    
    HttpResponse::Ok()
        .content_type("application/json")
        .json(json!({
            "generating": new_status,
            "message": format!("Product generation {}", if new_status { "started" } else { "stopped" })
        }))
}

async fn shutdown_server() -> impl Responder {
    println!("Shutting down server via /api/shutdown endpoint");
    let response = HttpResponse::Ok().json(serde_json::json!({"message": "Server is shutting down"}));
    // Give the response before exiting
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        std::process::exit(0);
    });
    response
}

async fn get_products_by_user_id(
    data: web::Data<AppState>,
    user_id: web::Path<i32>,
    query: web::Query<ProductQuery>,
) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    let mut products = repository::get_all_products_with_category(conn, user_id.into_inner()).unwrap_or_default();

    // Apply category filter
    if let Some(category_id) = query.category_id {
        products.retain(|p| p.category_id == category_id);
    }

    // Apply price filters
    if let Some(min_price) = query.min_price {
        products.retain(|p| p.price >= min_price);
    }
    if let Some(max_price) = query.max_price {
        products.retain(|p| p.price <= max_price);
    }

    // Apply search term
    if let Some(search_term) = &query.search_term {
        let search_term = search_term.to_lowercase();
        products.retain(|p| {
            p.name.to_lowercase().contains(&search_term) ||
            p.description.to_lowercase().contains(&search_term)
        });
    }

    // Apply sorting
    if let Some(sort_by) = &query.sort_by {
        let sort_order = query.sort_order.as_deref().unwrap_or("asc");
        products.sort_by(|a, b| {
            let cmp = match sort_by.as_str() {
                "name" => a.name.cmp(&b.name),
                "price" => a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal),
                _ => std::cmp::Ordering::Equal,
            };
            if sort_order == "desc" {
                cmp.reverse()
            } else {
                cmp
            }
        });
    }

    HttpResponse::Ok().json(products)
}

async fn monitor_logs_task(app_state: web::Data<AppState>) {
    use chrono::Utc;
    use crate::db::repository::{get_monitored_users, add_monitored_user, clear_monitored_users};
    use crate::db::schema::logs::dsl::*;
    use crate::db::schema::users::dsl as users_dsl;
    use diesel::prelude::*;
    use std::time::Duration;

    loop {
        let conn = &mut app_state.pool.get().unwrap();
        // Get logs from the last minute
        let now = Utc::now().naive_utc();
        let one_min_ago = now - chrono::Duration::minutes(1);
        let recent_logs: Vec<(i32, String)> = logs
            .filter(timestamp.ge(one_min_ago))
            .select((user_id, action))
            .load(conn)
            .unwrap_or_default();
        // Count actions per user
        let mut user_counts: HashMap<i32, i32> = HashMap::new();
        for (uid, _) in recent_logs {
            *user_counts.entry(uid).or_insert(0) += 1;
        }
        // For each suspicious user, add to monitored_users
        for (uid, count) in user_counts {
            if count > 10 {
                // Get username
                if let Ok(user) = users_dsl::users.find(uid).first::<crate::db::models::User>(conn) {
                    let _ = add_monitored_user(conn, uid, &user.username);
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

async fn get_monitored_users_handler(data: web::Data<AppState>) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    let monitored = crate::db::repository::get_monitored_users(conn).unwrap_or_default();
    HttpResponse::Ok().json(monitored)
}

#[derive(Deserialize)]
struct StatsQuery {
    user_id: i32,
}

async fn avg_price_per_category_handler(data: web::Data<AppState>, query: web::Query<StatsQuery>) -> impl Responder {
    use diesel::dsl::avg;
    use crate::db::schema::products::dsl as products_dsl;
    use crate::db::schema::categories::dsl as categories_dsl;
    let conn = &mut data.pool.get().unwrap();
    let user_id_val = query.user_id;
    type Row = (String, Option<f64>);
    let results: Vec<Row> = products_dsl::products
        .inner_join(categories_dsl::categories.on(products_dsl::category_id.eq(categories_dsl::id)))
        .filter(products_dsl::user_id.eq(user_id_val))
        .group_by(categories_dsl::name)
        .select((categories_dsl::name, avg(products_dsl::price)))
        .order_by(avg(products_dsl::price).desc())
        .load(conn)
        .unwrap_or_default();
    let response: Vec<_> = results.into_iter().map(|(category, avg_price)| serde_json::json!({"category": category, "avg_price": avg_price})).collect();
    HttpResponse::Ok().json(response)
}

#[derive(Deserialize)]
struct AvgPriceQuery {
    user_id: i32,
}

async fn avg_price_inefficient_handler(data: web::Data<AppState>, query: web::Query<AvgPriceQuery>) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    let user_id_val = query.user_id;
    // Load all products for the user
    let products = crate::db::repository::get_all_products(conn, user_id_val).unwrap_or_default();
    if products.is_empty() {
        return HttpResponse::Ok().json(serde_json::json!({"average_price": null, "count": 0}));
    }
    // Calculate average in Rust (inefficient)
    let sum: f64 = products.iter().map(|p| p.price).sum();
    let avg = sum / (products.len() as f64);
    HttpResponse::Ok().json(serde_json::json!({"average_price": avg, "count": products.len()}))
}

async fn avg_price_per_category_inefficient_handler(data: web::Data<AppState>, query: web::Query<StatsQuery>) -> impl Responder {
    let conn = &mut data.pool.get().unwrap();
    let user_id_val = query.user_id;
    
    // Load all products with their categories
    let products = repository::get_all_products_with_category(conn, user_id_val).unwrap_or_default();
    
    // Group products by category and calculate averages in memory
    let mut category_sums: HashMap<String, (f64, i32)> = HashMap::new();
    
    for product in products {
        let entry = category_sums.entry(product.category_name)
            .or_insert((0.0, 0));
        entry.0 += product.price;
        entry.1 += 1;
    }
    
    // Convert to final format and sort by average price
    let mut results: Vec<_> = category_sums.into_iter()
        .map(|(category, (sum, count))| {
            let avg_price = if count > 0 { Some(sum / count as f64) } else { None };
            serde_json::json!({
                "category": category,
                "avg_price": avg_price
            })
        })
        .collect();
    
    // Sort by average price in descending order
    results.sort_by(|a, b| {
        let a_price = a["avg_price"].as_f64().unwrap_or(0.0);
        let b_price = b["avg_price"].as_f64().unwrap_or(0.0);
        b_price.partial_cmp(&a_price).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    HttpResponse::Ok().json(results)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize the database connection pool
    db::connection::init_pool();
    
    // Initialize the app state with only the pool
    let app_state = web::Data::new(AppState {
        pool: r2d2::Pool::builder()
            .build(ConnectionManager::<PgConnection>::new("postgres://postgres:luscan@localhost:5432/postgres"))
            .expect("Failed to create pool"),
    });
    
    // Clone app_state for the background task
    let app_state_clone = app_state.clone();
    
    // Spawn the background task (if needed, or remove if not used)
    tokio::spawn(async move {
        // generate_products(app_state_clone).await;
    });

    // Spawn the background monitor task
    tokio::spawn(async move {
        monitor_logs_task(app_state_clone).await;
    });

    println!("Server running at http://localhost:3001");

    start_server().await
}