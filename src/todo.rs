use maud::{html, Markup};
use serde::Deserialize;
use sqlx::SqlitePool;

#[derive(Deserialize)]
pub struct Todo {
    pub id: i64,
    pub text: String,
    pub done: bool,
}

impl Todo {
    pub async fn fetch(pool: &SqlitePool, id: i64) -> Option<Self> {
        let res = sqlx::query_as!(Todo, "SELECT id, text, done FROM todos WHERE id = $1", id)
            .fetch_one(pool)
            .await;
        res.ok()
    }

    pub async fn fetch_all(pool: &SqlitePool, hide_done: bool) -> Option<Vec<Self>> {
        let res = if hide_done {
            sqlx::query_as!(
                Todo,
                "SELECT id, text, done FROM todos WHERE done == false ORDER BY time DESC",
            )
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as!(Todo, "SELECT id, text, done FROM todos ORDER BY time DESC")
                .fetch_all(pool)
                .await
        };
        res.ok()
    }

    pub async fn list_template(pool: &SqlitePool, hide_done: bool) -> Option<Markup> {
        let Some(todos) = Todo::fetch_all(&pool, hide_done).await else {
            return None;
        };

        Some(
            html! {
                @for todo in todos {
                    @if !hide_done || !todo.done {
                        (todo.partial())
                    }
                }
            }
            .into(),
        )
    }

    pub fn partial(&self) -> Markup {
        let id = format!("todo-{}", self.id);
        html! {
            div class="flex my-4" id=(id) {
                form class="flex flex-1" hx-patch={"/todo/" (self.id)} hx-target={"#" (id)} hx-swap="morph:outerHTML" {
                    button type="submit" class={"flex-1 mr-4 p-4 rounded-xl max-w-sm mx-auto cursor-pointer " @if self.done { "line-through bg-slate-400" } @else { "bg-white" } }
                    {
                        (self.text)
                    }
                    input type="hidden" name="text" value=(self.text) {}
                    input type="hidden" name="id" value=(self.id) {}
                    input type="hidden" name="done" value=(!self.done) {}
                }
                button class="text-white border text-2xl rounded-xl w-14 mx-auto" hx-trigger="click" hx-delete={"/todo/" (self.id)} hx-target={"#" (id)} hx-swap="outerHTML" { "x" }
            }
        }
    }
}
