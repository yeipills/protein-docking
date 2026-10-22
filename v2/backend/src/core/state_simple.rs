/*!
 * Estado de la aplicación simplificado
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub version: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            version: "2.0.0".to_string(),
            started_at: chrono::Utc::now(),
        }
    }
}