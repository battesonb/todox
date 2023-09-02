use axum::Router;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new().fallback_service(ServeDir::new("public"));

    axum::Server::bind(&"127.0.0.1:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
