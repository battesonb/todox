pub mod body;

use std::{collections::VecDeque, sync::Arc};

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
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

const HX_TRIGGER: &'static str = "HX-Trigger";

#[derive(Clone)]
struct Todo {
    id: usize,
    name: String,
    completed: bool,
}

impl Todo {
    pub fn new(id: usize, name: String) -> Self {
        Self {
            id,
            name,
            completed: false,
        }
    }

    pub fn partial(&self) -> Markup {
        let id = format!("todo-{}", self.id);
        html! {
            div class="flex my-4" id=(id) {
                form class="flex flex-1" hx-patch={"/todo/" (self.id)} hx-target={"#" (id)} hx-swap="morph:outerHTML" {
                    button type="submit" class={"flex-1 mr-4 p-4 rounded-xl max-w-sm mx-auto cursor-pointer " @if self.completed { "line-through bg-slate-400" } @else { "bg-white" } }
                    {
                        (self.name)
                    }
                    input type="hidden" name="id" value=(self.id) {}
                    input type="hidden" name="completed" value=(self.completed) {}
                }
                button class="text-white border text-2xl rounded-xl w-14 mx-auto" hx-trigger="click" hx-delete={"/todo/" (self.id)} hx-swap="none" { "x" }
            }
        }
    }
}

#[derive(Default)]
struct InnerState {
    next_id: usize,
    hide_completed: bool,
    todos: VecDeque<Todo>,
}

type AppState = Arc<RwLock<InnerState>>;

#[tokio::main]
async fn main() {
    let state = AppState::default();
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
}

async fn get_body(State(state): State<AppState>) -> Response {
    let state = state.read().await;
    body(state.hide_completed).into_string().into_response()
}

async fn delete_todo(State(state): State<AppState>, Path(todo_id): Path<usize>) -> Response {
    let mut state = state.write().await;
    let todos = &mut state.todos;
    let Some(index) = todos.iter().position(|t| t.id == todo_id) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    todos.remove(index);

    let mut headers = HeaderMap::new();
    headers.insert(HX_TRIGGER, "modifiedPosts".parse().unwrap());
    headers.into_response()
}

async fn delete_completed(State(state): State<AppState>) -> Response {
    let mut state = state.write().await;
    let todos = &mut state.todos;
    let len = todos.len();
    todos.retain(|t| !t.completed);

    if len != todos.len() {
        return StatusCode::NOT_FOUND.into_response();
    }

    ().into_response()
}

#[derive(Deserialize)]
struct PartialTodo {
    id: usize,
    completed: bool,
}

async fn patch_todo(State(state): State<AppState>, Form(form): Form<PartialTodo>) -> Response {
    let mut state = state.write().await;
    let todos = &mut state.todos;
    let Some(todo) = todos.iter_mut().find(|t| t.id == form.id) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    todo.completed = !form.completed;

    let mut response = todo.partial().into_string().into_response();
    if todo.completed {
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
    let mut state = state.write().await;

    if post_form.text.len() == 0 {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let todo = Todo::new(state.next_id, post_form.text);
    state.next_id += 1;
    state.todos.push_front(todo);

    let mut headers = HeaderMap::new();
    headers.insert(HX_TRIGGER, "modifiedPosts".parse().unwrap());
    headers.into_response()
}

async fn get_todos(State(state): State<AppState>) -> impl IntoResponse {
    let state = state.read().await;
    let todos = &state.todos;

    Html::<String>(
        html! {
            @for todo in todos {
                @if !state.hide_completed || !todo.completed  {
                    (todo.partial())
                }
            }
        }
        .into(),
    )
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
