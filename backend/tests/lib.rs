use actix_web::{test, web, App};
use super::*;

#[actix_web::test]
async fn test_get_products() {
    let app_state = web::Data::new(AppState {
        products: Mutex::new(mock_data::init_mock_data()),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/api/get/products", web::get().to(get_products))
    ).await;

    let req = test::TestRequest::get().uri("/api/get/products").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_create_product() {
    let app_state = web::Data::new(AppState {
        products: Mutex::new(Vec::new()),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/api/post/products", web::post().to(create_product))
    ).await;

    let new_product = CreateProductRequest {
        name: "Test Product".to_string(),
        price: 99.99,
        image: "test.jpg".to_string(),
        description: "Test Description".to_string(),
        category: "Test Category".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/post/products")
        .set_json(&new_product)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_create_product_validation() {
    let app_state = web::Data::new(AppState {
        products: Mutex::new(Vec::new()),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/api/post/products", web::post().to(create_product))
    ).await;

    // Test with empty name
    let invalid_product = CreateProductRequest {
        name: "".to_string(),
        price: 99.99,
        image: "test.jpg".to_string(),
        description: "Test Description".to_string(),
        category: "Test Category".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/post/products")
        .set_json(&invalid_product)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());
}

#[actix_web::test]
async fn test_update_product() {
    let mut mock_products = Vec::new();
    mock_products.push(Product {
        id: 1,
        name: "Original Product".to_string(),
        price: 50.0,
        image: "original.jpg".to_string(),
        description: "Original Description".to_string(),
        category: "Original Category".to_string(),
    });

    let app_state = web::Data::new(AppState {
        products: Mutex::new(mock_products),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/api/products/{id}", web::put().to(update_product))
    ).await;

    let updated_product = Product {
        id: 1,
        name: "Updated Product".to_string(),
        price: 75.0,
        image: "updated.jpg".to_string(),
        description: "Updated Description".to_string(),
        category: "Updated Category".to_string(),
    };

    let req = test::TestRequest::put()
        .uri("/api/products/1")
        .set_json(&updated_product)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_delete_product() {
    let mut mock_products = Vec::new();
    mock_products.push(Product {
        id: 1,
        name: "Test Product".to_string(),
        price: 50.0,
        image: "test.jpg".to_string(),
        description: "Test Description".to_string(),
        category: "Test Category".to_string(),
    });

    let app_state = web::Data::new(AppState {
        products: Mutex::new(mock_products),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/api/products/{id}", web::delete().to(delete_product))
    ).await;

    let req = test::TestRequest::delete().uri("/api/products/1").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_filter_and_sort_products() {
    let mut mock_products = Vec::new();
    mock_products.push(Product {
        id: 1,
        name: "Product A".to_string(),
        price: 100.0,
        image: "a.jpg".to_string(),
        description: "Description A".to_string(),
        category: "Category A".to_string(),
    });
    mock_products.push(Product {
        id: 2,
        name: "Product B".to_string(),
        price: 50.0,
        image: "b.jpg".to_string(),
        description: "Description B".to_string(),
        category: "Category B".to_string(),
    });

    let app_state = web::Data::new(AppState {
        products: Mutex::new(mock_products),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/api/products", web::get().to(get_products))
    ).await;

    // Test filtering by category
    let req = test::TestRequest::get()
        .uri("/api/products?category=Category%20A")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Test sorting by price
    let req = test::TestRequest::get()
        .uri("/api/products?sort_by=price&sort_order=asc")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_get_nonexistent_product() {
    let app_state = web::Data::new(AppState {
        products: Mutex::new(Vec::new()),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/api/products/{id}", web::get().to(get_product))
    ).await;

    let req = test::TestRequest::get().uri("/api/products/999").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());
}

#[actix_web::test]
async fn test_patch_product_validation() {
    let mut mock_products = Vec::new();
    mock_products.push(Product {
        id: 1,
        name: "Original Product".to_string(),
        price: 50.0,
        image: "original.jpg".to_string(),
        description: "Original Description".to_string(),
        category: "Original Category".to_string(),
    });

    let app_state = web::Data::new(AppState {
        products: Mutex::new(mock_products),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/api/patch/products/{id}", web::patch().to(patch_product))
    ).await;

    // Test with invalid data
    let invalid_update = CreateProductRequest {
        name: "".to_string(), // Empty name
        price: -1.0, // Negative price
        image: "".to_string(), // Empty image
        description: "".to_string(), // Empty description
        category: "".to_string(), // Empty category
    };

    let req = test::TestRequest::patch()
        .uri("/api/patch/products/1")
        .set_json(&invalid_update)
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());
}

#[actix_web::test]
async fn test_filter_products_edge_cases() {
    let mut mock_products = Vec::new();
    mock_products.push(Product {
        id: 1,
        name: "Product A".to_string(),
        price: 100.0,
        image: "a.jpg".to_string(),
        description: "Description A".to_string(),
        category: "Category A".to_string(),
    });
    mock_products.push(Product {
        id: 2,
        name: "Product B".to_string(),
        price: 50.0,
        image: "b.jpg".to_string(),
        description: "Description B".to_string(),
        category: "Category B".to_string(),
    });

    let app_state = web::Data::new(AppState {
        products: Mutex::new(mock_products),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/api/get/products", web::get().to(get_products))
    ).await;

    // Test with invalid sort field
    let req = test::TestRequest::get()
        .uri("/api/get/products?sort_by=invalid_field&sort_order=asc")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Test with invalid sort order
    let req = test::TestRequest::get()
        .uri("/api/get/products?sort_by=price&sort_order=invalid_order")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Test with min_price > max_price
    let req = test::TestRequest::get()
        .uri("/api/get/products?min_price=200&max_price=100")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let body = test::read_body(resp).await;
    let products: Vec<Product> = serde_json::from_slice(&body).unwrap();
    assert!(products.is_empty());
}

#[actix_web::test]
async fn test_delete_nonexistent_product() {
    let app_state = web::Data::new(AppState {
        products: Mutex::new(Vec::new()),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/api/delete/products/{id}", web::delete().to(delete_product))
    ).await;

    let req = test::TestRequest::delete().uri("/api/delete/products/999").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error());
} 