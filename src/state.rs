use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

const USER_STATE: &'static str = "user_state";

#[derive(Default, Deserialize, Serialize)]
pub struct UserState {
    pub hide_done: bool,
}

impl UserState {
    pub async fn new(pool: &SqlitePool) -> Self {
        let Ok(result) = sqlx::query!("SELECT data FROM state WHERE id = $1", USER_STATE)
            .fetch_one(pool)
            .await else {
            let state = Self::default();
            let json = serde_json::to_string(&state).unwrap();
            // seed value; should probably do something with this Result.
            let _ = sqlx::query!("INSERT INTO state(id, data) VALUES ($1, $2)", USER_STATE, json)
                .execute(pool)
                .await;
            return state;
        };

        serde_json::from_str(&result.data).unwrap_or_default()
    }

    pub async fn save(&self, pool: &SqlitePool) -> bool {
        let json = serde_json::to_string(self).unwrap();
        let Ok(_) = sqlx::query!("UPDATE state SET data = $1 WHERE id = $2", json, USER_STATE)
            .execute(pool)
            .await else {
            return false;
        };

        true
    }
}
