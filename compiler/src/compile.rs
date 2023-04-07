//! The complete `/compile` route

use super::{APP_CONTAINER_TAG, CODE_FILE_NAME, OUTPUT_WASM_NAME};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

#[derive(Debug, Deserialize)]
pub(crate) struct CompileInfo {
    pub(crate) code: String,
}

pub(crate) async fn compile(Json(input): Json<CompileInfo>) -> Result<Vec<u8>, CompileError> {
    let code_dir = tempfile::tempdir()?;
    let code_dir_path = code_dir.path();

    let mut code_file = File::create(code_dir_path.join(CODE_FILE_NAME)).await?;
    code_file.write_all(input.code.as_bytes()).await?;

    let code_dir_path_str = code_dir_path.to_string_lossy();
    let time = std::time::Instant::now();
    let command_status = Command::new("docker")
        .args([
            "run",
            // "--rm",
            "-v",
            &format!("{code_dir_path_str}:/usr/src/app/src/"),
            APP_CONTAINER_TAG,
        ])
        .output()
        .await?;

    println!("Elapsed: {:.2?}", time.elapsed());

    if !command_status.status.success() {
        return Err(CompileError::Compile(
            String::from_utf8(command_status.stderr).unwrap(),
        ));
    }

    let mut wasm = Vec::new();
    File::open(code_dir_path.join(OUTPUT_WASM_NAME))
        .await?
        .read_to_end(&mut wasm)
        .await?;

    Ok(wasm)
}

pub(crate) enum CompileError {
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
pub(crate) struct Error {
    pub(crate) msg: &'static str,
    pub(crate) full: String,
}
