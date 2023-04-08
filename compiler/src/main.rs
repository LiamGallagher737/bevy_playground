use crate::docker_pool::DockerPool;
use axum::{extract::DefaultBodyLimit, handler::Handler, routing::post, Extension, Router};
use futures::future::join_all;
use std::net::SocketAddr;
use tokio::{process::Command, signal};

const CODE_FILE_NAME: &str = "main.rs";
const OUTPUT_WASM_NAME: &str = "app.wasm";
const APP_CONTAINER_TAG: &str = "liamg737/bevy_playground_app:0.0.1";
const APP_CONTAINER_RELATIVE_DIR: &str = "./app";
const BODY_SIZE_LIMIT: usize = 250_000; // 0.25 MB

const MIN_READY_CONTAINERS: usize = 1;
const MAX_READY_CONTAINERS: usize = 3;

const TEMP_PATH: &str = ".bevy_playground";

mod compile;
mod docker_pool;

#[tokio::main]
async fn main() {
    // Replace with docker image pull APP_CONTAINER_TAG
    let image_build_command = Command::new("docker")
        .args(["build", "-t", APP_CONTAINER_TAG, APP_CONTAINER_RELATIVE_DIR])
        .status()
        .await
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

    let docker_pool = DockerPool::new(MIN_READY_CONTAINERS).await;

    let app = Router::new()
        .route(
            "/compile",
            post(compile::compile.layer(DefaultBodyLimit::max(BODY_SIZE_LIMIT))),
        )
        .layer(Extension(docker_pool));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Listening on {}", addr);

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
        .args(&["container", "ls", "-q", "--filter", "name=bp-dp.*"])
        .output()
        .await
        .expect("Failed to list matching containers");

    let stdout = String::from_utf8(out.stdout).unwrap();
    let containers: Vec<_> = stdout.lines().collect();

    let mut commands = Vec::with_capacity(containers.len());
    for id in containers {
        let cmd = Command::new("docker").args(&["stop", id]).status();
        commands.push(cmd);
    }
    let status = join_all(commands).await;

    if status
        .iter()
        .any(|s| s.is_err() || !s.as_ref().unwrap().success())
    {
        eprintln!("Error: One or more compiler containers failed to stop!");
    }

    let temp_dir = std::env::temp_dir();
    tokio::fs::remove_dir_all(temp_dir.join(TEMP_PATH))
        .await
        .expect("Failed to remove temp directory");
}
