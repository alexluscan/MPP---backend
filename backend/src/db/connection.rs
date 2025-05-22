use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use config::Config;
use std::sync::OnceLock;

pub type PgPool = Pool<ConnectionManager<PgConnection>>;
pub type PgPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

static POOL: OnceLock<PgPool> = OnceLock::new();

pub fn init_pool() -> PgPool {
    let settings = Config::builder()
        .add_source(config::File::with_name("appsettings"))
        .build()
        .expect("Failed to load configuration");

    let database_url = settings.get_string("database.url").expect("Database URL not found");
    let pool_size = settings.get_int("database.pool_size").unwrap_or(10) as u32;
    let timeout = settings.get_int("database.timeout_seconds").unwrap_or(30) as u64;

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