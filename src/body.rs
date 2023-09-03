use maud::{html, Markup};
use sqlx::SqlitePool;

use crate::todo::Todo;

pub fn toggle_completed_button(completed: bool) -> Markup {
    html! {
        button class={"flex-1 rounded-xl p-4 mr-4 " @if completed { "bg-slate-400" } @else { "bg-white" }} hx-post="/toggle-completed" hx-swap="outerHTML" { "Hide completed" }
    }
}

pub async fn body(pool: &SqlitePool, hide_done: bool) -> Option<Markup> {
    let Some(todos) = Todo::list_template(pool, hide_done).await else {
        return None;
    };

    Some(html! {
        div hx-ext="morph" class="max-w-md mx-auto m-6" {
          h1 class="text-white text-6xl text-center" { "todox" }
          div class="flex my-4" {
            (toggle_completed_button(hide_done))
            button class="flex-1 bg-white rounded-xl p-4" hx-swap="none" hx-delete="/todo" { "Delete completed" }
          }
          form class="flex my-4" hx-target="#todos" hx-post="/todo" _="on htmx:afterOnLoad me.reset()" hx-swap="afterbegin" {
            input name="text" class="flex-1 bg-white mr-4 p-4 rounded-xl max-w-sm mx-auto" {}
            button type="submit" class="bg-white text-2xl rounded-xl w-14 mx-auto" { "+" }
          }
          div id="todos" hx-trigger="modifiedPosts from:body" hx-get="/todos" hx-swap="morph:innerHTML" {
              (todos)
          }
        }
    })
}
