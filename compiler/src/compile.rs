//! The complete `/compile` route

use super::{CODE_FILE_NAME, OUTPUT_WASM_NAME};
use crate::docker_pool::DockerPool;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

#[derive(Debug, Deserialize)]
pub struct CompileInfo {
    code: String,
}

pub async fn compile(
    Extension(container_pool): Extension<DockerPool>,
    Json(input): Json<CompileInfo>,
) -> Result<Vec<u8>, CompileError> {
    let (id, container) = container_pool.take().await;

    let mut code_file = File::create(container.directory.join(CODE_FILE_NAME)).await?;
    code_file.write_all(input.code.as_bytes()).await?;

    // let time = std::time::Instant::now();
    let command_status = Command::new("docker")
        .args([
            "exec",
            &container.name,
            "sh",
            "-c",
            "cargo build --release && mv target/wasm32-unknown-unknown/release/app.wasm src/app.wasm"
        ])
        .output()
        .await?;
    // println!("Elapsed: {:.2?}", time.elapsed());

    if !command_status.status.success() {
        return Err(CompileError::Compile(
            String::from_utf8(command_status.stdout).unwrap(),
        ));
    }

    let mut wasm = Vec::new();
    File::open(container.directory.join(OUTPUT_WASM_NAME))
        .await?
        .read_to_end(&mut wasm)
        .await?;

    container_pool.release(id).await;

    Ok(wasm)
}

pub enum CompileError {
    Io(std::io::Error),
    Compile(String),
}

impl From<std::io::Error> for CompileError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl IntoResponse for CompileError {
    fn into_response(self) -> Response {
        match self {
            CompileError::Io(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(Error {
                    msg: "Io error",
                    full: format!("{e:#?}"),
                }),
            ),
            CompileError::Compile(stderr) => (
                StatusCode::BAD_REQUEST,
                Json(Error {
                    msg: "Could not compile app",
                    full: stderr,
                }),
            ),
        }
        .into_response()
    }
}

#[derive(Serialize)]
struct Error {
    msg: &'static str,
    full: String,
}
