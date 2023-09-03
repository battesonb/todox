pub mod body;
pub mod state;
pub mod todo;

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, patch, post},
    Form, Router,
};
use body::{body, toggle_completed_button};
use serde::Deserialize;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use state::UserState;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use crate::todo::Todo;

const HX_TRIGGER: &'static str = "HX-Trigger";

#[derive(Clone)]
struct InnerState {
    pool: SqlitePool,
}

type AppState = Arc<RwLock<InnerState>>;

impl InnerState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let options = SqliteConnectOptions::new()
            .filename("db.sqlite")
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        Ok(Self { pool })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(RwLock::new(InnerState::new().await?));
    let app = Router::new()
        .route("/body", get(get_body))
        .route("/todos", get(get_todos))
        .route("/todo", post(post_todo))
        .route("/todo", delete(delete_completed))
        .route("/todo/:todo_id", delete(delete_todo))
        .route("/todo/:todo_id", patch(patch_todo))
        .route("/toggle-completed", post(toggle_completed))
        .with_state(state)
        .fallback_service(ServeDir::new("public"));

    axum::Server::bind(&"127.0.0.1:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn get_body(State(state): State<AppState>) -> Response {
    let state = state.read().await;
    let user_state = UserState::new(&state.pool).await;
    let Some(body) = body(&state.pool, user_state.hide_done).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    body.into_string().into_response()
}

async fn delete_todo(State(state): State<AppState>, Path(todo_id): Path<i64>) -> Response {
    let state = state.read().await;

    let Ok(output) = sqlx::query("DELETE FROM todos WHERE id = $1")
        .bind(todo_id)
        .execute(&state.pool)
        .await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    if output.rows_affected() == 0 {
        return StatusCode::NOT_FOUND.into_response();
    }

    ().into_response()
}

async fn delete_completed(State(state): State<AppState>) -> Response {
    let state = state.read().await;

    let Ok(output) = sqlx::query("DELETE FROM todos WHERE done = true")
        .execute(&state.pool)
        .await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    if output.rows_affected() == 0 {
        return ().into_response();
    }

    let mut headers = HeaderMap::new();
    headers.insert(HX_TRIGGER, "modifiedPosts".parse().unwrap());
    headers.into_response()
}

async fn patch_todo(State(state): State<AppState>, Form(form): Form<Todo>) -> Response {
    let state = state.read().await;
    let user_state = UserState::new(&state.pool).await;

    let Ok(output) = sqlx::query!("UPDATE todos SET done = $1, text = $2 WHERE id = $3", form.done, form.text, form.id)
        .execute(&state.pool)
        .await else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    if output.rows_affected() == 0 {
        return StatusCode::NOT_FOUND.into_response();
    }

    let Some(todo) = Todo::fetch(&state.pool, form.id).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    if todo.done && user_state.hide_done {
        ().into_response()
    } else {
        todo.partial().into_string().into_response()
    }
}

#[derive(Deserialize)]
struct PostForm {
    text: String,
}

async fn post_todo(State(state): State<AppState>, Form(post_form): Form<PostForm>) -> Response {
    if post_form.text.len() == 0 {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let state = state.read().await;
    let Ok(res) = sqlx::query("INSERT INTO todos(text) VALUES ($1)")
        .bind(post_form.text)
        .execute(&state.pool)
        .await else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let id = res.last_insert_rowid();
    let Some(todo) = Todo::fetch(&state.pool, id).await else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    todo.partial().into_string().into_response()
}

async fn get_todos(State(state): State<AppState>) -> Response {
    let state = state.read().await;
    let user_state = UserState::new(&state.pool).await;

    let Some(markup) = Todo::list_template(&state.pool, user_state.hide_done).await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    Html(markup.into_string()).into_response()
}

async fn toggle_completed(State(state): State<AppState>) -> Response {
    let state = state.write().await;
    let mut user_state = UserState::new(&state.pool).await;
    user_state.hide_done = !user_state.hide_done;
    if !user_state.save(&state.pool).await {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let markup = toggle_completed_button(user_state.hide_done);
    let mut response = markup.into_string().into_response();

    let headers = response.headers_mut();
    headers.insert(HX_TRIGGER, "modifiedPosts".parse().unwrap());

    response
}
