pub mod body;

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, patch, post},
    Form, Router,
};
use body::{body, toggle_completed_button};
use maud::{html, Markup};
use serde::Deserialize;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

const HX_TRIGGER: &'static str = "HX-Trigger";

#[derive(Clone)]
struct Todo {
    id: i64,
    text: String,
    done: bool,
}

impl Todo {
    pub fn partial(&self) -> Markup {
        let id = format!("todo-{}", self.id);
        html! {
            div class="flex my-4" id=(id) {
                form class="flex flex-1" hx-patch={"/todo/" (self.id)} hx-target={"#" (id)} hx-swap="morph:outerHTML" {
                    button type="submit" class={"flex-1 mr-4 p-4 rounded-xl max-w-sm mx-auto cursor-pointer " @if self.done { "line-through bg-slate-400" } @else { "bg-white" } }
                    {
                        (self.text)
                    }
                    input type="hidden" name="id" value=(self.id) {}
                    input type="hidden" name="done" value=(self.done) {}
                }
                button class="text-white border text-2xl rounded-xl w-14 mx-auto" hx-trigger="click" hx-delete={"/todo/" (self.id)} hx-target={"#" (id)} hx-swap="outerHTML" { "x" }
            }
        }
    }
}

struct InnerState {
    hide_completed: bool,
    pool: SqlitePool,
}

type AppState = Arc<RwLock<InnerState>>;

impl InnerState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let options = SqliteConnectOptions::new()
            .filename("db.sqlite")
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;
        Ok(Self {
            hide_completed: false,
            pool,
        })
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
    body(state.hide_completed).into_string().into_response()
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

#[derive(Deserialize)]
struct PartialTodo {
    id: i64,
    done: bool,
}

async fn patch_todo(State(state): State<AppState>, Form(form): Form<PartialTodo>) -> Response {
    let state = state.read().await;

    let done = !form.done;
    let Ok(output) = sqlx::query!("UPDATE todos SET done = $1 WHERE id = $2", done, form.id)
        .execute(&state.pool)
        .await else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    if output.rows_affected() == 0 {
        return StatusCode::NOT_FOUND.into_response();
    }

    let Ok(todo) = sqlx::query_as!(Todo, "SELECT id, text, done FROM todos WHERE id = $1", form.id)
        .fetch_one(&state.pool)
        .await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let mut response = todo.partial().into_string().into_response();
    if todo.done {
        let headers = response.headers_mut();
        headers.insert(HX_TRIGGER, "modifiedPosts".parse().unwrap());
    }
    response
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
    let Ok(_) = sqlx::query("INSERT INTO todos(text) VALUES ($1)")
        .bind(post_form.text)
        .execute(&state.pool)
        .await else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let mut headers = HeaderMap::new();
    headers.insert(HX_TRIGGER, "modifiedPosts".parse().unwrap());
    headers.into_response()
}

async fn get_todos(State(state): State<AppState>) -> Response {
    let state = state.read().await;
    let Ok(todos) = sqlx::query_as!(Todo, "SELECT id, text, done FROM todos ORDER BY time DESC")
        .fetch_all(&state.pool)
        .await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    Html::<String>(
        html! {
            @for todo in todos {
                @if !state.hide_completed || !todo.done {
                    (todo.partial())
                }
            }
        }
        .into(),
    )
    .into_response()
}

async fn toggle_completed(State(state): State<AppState>) -> impl IntoResponse {
    let mut state = state.write().await;
    state.hide_completed = !state.hide_completed;

    let markup = toggle_completed_button(state.hide_completed);
    let mut response = markup.into_string().into_response();

    let headers = response.headers_mut();
    headers.insert(HX_TRIGGER, "modifiedPosts".parse().unwrap());

    response
}
