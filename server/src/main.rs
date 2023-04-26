#![warn(clippy::pedantic)]

use crate::docker_pool::DockerPool;
use axum::{
    extract::DefaultBodyLimit,
    handler::Handler,
    http::{Method},
    routing::post,
    Extension, Router,
};
use std::net::SocketAddr;
use tokio::{process::Command, signal};
use tower_http::cors::{Any, CorsLayer};

const CODE_FILE_NAME: &str = "main.rs";
const OUTPUT_WASM_NAME: &str = "game.wasm";
const OUTPUT_WASM_NAME_BG: &str = "game_bg.wasm";
const CONTAINER_TAG: &str = "liamg737/bevy_playground_compiler_instance:0.0.1";
const CONTAINER_RELATIVE_DIR: &str = "./compiler_instance";
const CONTAINER_PREFIX: &str = "bp-dp";
const BODY_SIZE_LIMIT: usize = 250_000; // 0.25 MB

const INITIAL_READY_CONTAINERS: usize = (MIN_READY_CONTAINERS + MAX_READY_CONTAINERS) / 2;
const MIN_READY_CONTAINERS: usize = 1;
const MAX_READY_CONTAINERS: usize = 3;

const TEMP_PATH: &str = ".bevy_playground";

mod compile;
mod docker_pool;

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT")
        .expect("Failed to get enviroment variable 'PORT'")
        .parse::<u16>()
        .expect("Failed to parse enviroment variable 'PORT' to type u16");

    // Replace with docker image pull APP_CONTAINER_TAG
    let image_build_command = Command::new("docker")
        .args(["build", "-t", CONTAINER_TAG, CONTAINER_RELATIVE_DIR])
        .status()
        .await
        .expect("Failed to run build command for compiler docker image");

    assert!(
        image_build_command.success(),
        "Compiler container image build failed! Error: {image_build_command:#?}"
    );

    let docker_pool = DockerPool::new(INITIAL_READY_CONTAINERS).await;

    let app = Router::new()
        .route(
            "/compile",
            post(compile::compile.layer(DefaultBodyLimit::max(BODY_SIZE_LIMIT))),
        )
        .layer(Extension(docker_pool))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Method::POST)
                .allow_headers(Any),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Listening on {addr}");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("Shutting down gracefully");

    let out = Command::new("docker")
        .args([
            "container",
            "ls",
            "-q",
            "--filter",
            &format!("name={CONTAINER_PREFIX}.*"),
        ])
        .output()
        .await
        .expect("Failed to list matching containers");

    let stdout = String::from_utf8(out.stdout).unwrap();
    let containers: Vec<_> = stdout.lines().collect();
    let status = Command::new("docker")
        .arg("kill")
        .args(containers)
        .status()
        .await;

    if status.is_err() || !status.as_ref().unwrap().success() {
        eprintln!("Error: One or more compiler containers failed to stop!");
    }

    let temp_dir = std::env::temp_dir();
    tokio::fs::remove_dir_all(temp_dir.join(TEMP_PATH))
        .await
        .expect("Failed to remove temp directory");
}
