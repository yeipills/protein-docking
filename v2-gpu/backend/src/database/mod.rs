// Database Module - Sistema de persistencia PostgreSQL ultra-optimizado
// Conexiones pooled, transacciones, migraciones automáticas

// Modelos y conexión completa (requiere PostgreSQL)
#[cfg(feature = "postgres")]
pub mod schema;
#[cfg(feature = "postgres")]
pub mod models;
#[cfg(feature = "postgres")]
pub mod connection;

// Modelos simplificados para desarrollo
pub mod simple_models;
pub mod simple_connection;

// Re-exports condicionales
#[cfg(feature = "postgres")]
pub use connection::*;
#[cfg(feature = "postgres")]
pub use models::*;
#[cfg(feature = "postgres")]
pub use schema::*;
#[cfg(feature = "postgres")]
pub use sqlx::{PgPool, Row, Transaction, Postgres};

// Re-exports para desarrollo
#[cfg(not(feature = "postgres"))]
pub use simple_connection::*;
#[cfg(not(feature = "postgres"))]
pub use simple_models::*;