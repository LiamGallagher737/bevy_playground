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
pub struct Body {
    code: String,
}

pub async fn compile(
    Extension(container_pool): Extension<DockerPool>,
    Json(body): Json<Body>,
) -> Result<Wasm, Error> {
    let (id, container) = container_pool.take().await;

    let mut code_file = File::create(container.directory.join(CODE_FILE_NAME)).await?;
    code_file.write_all(body.code.as_bytes()).await?;

    let command_status = Command::new("docker")
        .args([
            "exec",
            &container.name,
            "sh",
            "-c",
            &format!(
                "cargo build --release && mv target/wasm32-unknown-unknown/release/{OUTPUT_WASM_NAME} src/{OUTPUT_WASM_NAME}",
            ),
        ])
        .output()
        .await?;

    if !command_status.status.success() {
        return Err(Error::Compile(
            String::from_utf8(command_status.stdout).unwrap(),
        ));
    }

    let mut file = File::open(container.directory.join(OUTPUT_WASM_NAME)).await?;
    let mut wasm = Vec::with_capacity(file.metadata().await?.len() as usize);
    file.read_to_end(&mut wasm).await?;

    container_pool.release(id).await;

    Ok(Wasm(wasm))
}

pub struct ContentType<T> {
    content_type: &'static str,
    body: T,
}

impl<T: IntoResponse> IntoResponse for ContentType<T> {
    fn into_response(self) -> Response {
        ([("content-type", self.content_type)], self.body).into_response()
    }
}

pub struct Wasm(Vec<u8>);

impl IntoResponse for Wasm {
    fn into_response(self) -> Response {
        ([("content-type", "application/wasm")], self.0).into_response()
    }
}

pub enum Error {
    Io(std::io::Error),
    Compile(String),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Io(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    msg: "Io error",
                    full: format!("{e:#?}"),
                }),
            ),
            Self::Compile(stderr) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    msg: "Could not compile app",
                    full: stderr,
                }),
            ),
        }
        .into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    msg: &'static str,
    full: String,
}
