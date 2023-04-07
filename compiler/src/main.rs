use axum::{extract::DefaultBodyLimit, handler::Handler, routing::post, Router};
use std::net::SocketAddr;

const CODE_FILE_NAME: &str = "main.rs";
const OUTPUT_WASM_NAME: &str = "app.wasm";
const APP_CONTAINER_TAG: &str = "liamg737/bevy_playground_app:0.0.1";
const APP_CONTAINER_RELATIVE_DIR: &str = "./app";
const BODY_SIZE_LIMIT: usize = 250_000; // 0.25 MB

mod compile;
mod docker_pool;

#[tokio::main]
async fn main() {
    // Replace with docker image pull APP_CONTAINER_TAG
    let image_build_command = std::process::Command::new("docker")
        .args(["build", "-t", APP_CONTAINER_TAG, APP_CONTAINER_RELATIVE_DIR])
        .status()
        .expect("Failed to run build command for compiler docker image");

    if !image_build_command.success() {
        panic!(
            "Compiler container image build failed! Error: {:#?}",
            image_build_command
        );
    }

    let port = std::env::var("PORT")
        .expect("Failed to get enviroment variable 'PORT'")
        .parse::<u16>()
        .expect("Failed to parse enviroment variable 'PORT' to type u16");

    let app = Router::new().route(
        "/compile",
        post(compile::compile.layer(DefaultBodyLimit::max(BODY_SIZE_LIMIT))),
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
