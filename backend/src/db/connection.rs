use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use config::Config;
use std::sync::OnceLock;
use std::env;

pub type PgPool = Pool<ConnectionManager<PgConnection>>;
pub type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

static POOL: OnceLock<PgPool> = OnceLock::new();

pub fn init_pool() -> PgPool {
    // First try to get DATABASE_URL from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| {
            // Fallback to config file if DATABASE_URL is not set
            let settings = Config::builder()
                .add_source(config::File::with_name("appsettings"))
                .build()
                .expect("Failed to load configuration");
            settings.get_string("database.url").expect("Database URL not found")
        });

    let pool_size = env::var("DATABASE_POOL_SIZE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(10);

    let timeout = env::var("DATABASE_TIMEOUT_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .max_size(pool_size)
        .connection_timeout(std::time::Duration::from_secs(timeout))
        .build(manager)
        .expect("Failed to create pool")
}

pub fn get_pool() -> &'static PgPool {
    POOL.get_or_init(init_pool)
}

pub fn get_conn() -> PgPooledConnection {
    get_pool().get().expect("Failed to get connection from pool")
} 