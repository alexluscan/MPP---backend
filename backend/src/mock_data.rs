use crate::models::Product;
use chrono::Utc;

pub fn init_mock_data() -> Vec<Product> {
    vec![
        Product {
            id: 1,
            name: "Sample Product 1".to_string(),
            price: 99.99,
            description: "This is a sample product".to_string(),
            image: "/assets/images/placeholder.jpg".to_string(),
            video: None,
            category_id: 1,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        },
        Product {
            id: 2,
            name: "Sample Product 2".to_string(),
            price: 149.99,
            description: "Another sample product".to_string(),
            image: "/assets/images/placeholder.jpg".to_string(),
            video: None,
            category_id: 1,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        },
    ]
} 