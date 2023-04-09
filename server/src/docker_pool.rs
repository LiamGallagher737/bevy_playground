//! Starting a new container every request adds unwanted
//! overhead so we instead keep them alive to be reused

use crate::{
    CONTAINER_PREFIX, CONTAINER_TAG, MAX_READY_CONTAINERS, MIN_READY_CONTAINERS, TEMP_PATH,
};
use futures::future::join_all;
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering::Relaxed},
        Arc,
    },
};
use tokio::{process::Command, sync::RwLock};

#[derive(Default, Clone)]
pub struct DockerPool {
    next_id: Arc<AtomicUsize>,
    ready: Arc<RwLock<VecDeque<usize>>>,
    containers: Arc<RwLock<HashMap<usize, Container>>>,
}

#[derive(Clone)]
pub struct Container {
    pub name: String,
    pub directory: PathBuf,
}

impl DockerPool {
    /// Create a new pool with `count` containers
    pub async fn new(count: usize) -> Self {
        let pool = Self::default();
        pool.reserve(count).await;
        pool
    }

    /// Takes a container off the ready returning it's id and container info
    pub async fn take(&self) -> (usize, Container) {
        if self.ready.read().await.is_empty() {
            self.reserve(1).await;
        }

        let ready_count = self.ready.read().await.len();
        if ready_count - 1 < MIN_READY_CONTAINERS {
            let clone = self.clone();
            tokio::spawn(async move {
                clone.reserve(MIN_READY_CONTAINERS + 1 - ready_count).await;
            });
        }

        let id = self.ready.write().await.pop_front().unwrap();
        (id, self.containers.read().await[&id].clone())
    }

    /// Return a continer to be used again
    pub async fn release(&self, id: usize) {
        if self.ready.read().await.len() >= MAX_READY_CONTAINERS {
            let container = self.containers.write().await.remove(&id).unwrap();
            let _ = Command::new("docker")
                .args(["stop", &container.name])
                .status()
                .await;
            let _ = tokio::fs::remove_dir_all(container.directory).await;
            return;
        }
        self.ready.write().await.push_back(id);
        let path = &self.containers.read().await[&id].directory;
        let _ = tokio::fs::remove_dir_all(path).await;
        let _ = tokio::fs::create_dir(path).await;
    }

    /// Reserve some more containers
    pub async fn reserve(&self, count: usize) {
        let temp_dir = std::env::temp_dir();

        let mut commands = Vec::with_capacity(count);
        for _ in 0..count {
            let id = self.next_id.fetch_add(1, Relaxed);
            let name = format!("{CONTAINER_PREFIX}.{id}");

            let directory = temp_dir.join(TEMP_PATH).join(&name);
            let _ = tokio::fs::remove_dir_all(&directory).await;
            tokio::fs::create_dir_all(&directory).await.unwrap();

            // let ready = self.ready.clone();
            // let containers = self.containers.clone();
            let task = async move {
                let status = Command::new("docker")
                    .args([
                        "run",
                        "--name",
                        &name,
                        "--rm",
                        "-d",
                        "-i",
                        "-t",
                        "-v",
                        &format!("{}:/usr/src/app/src/", directory.display()),
                        CONTAINER_TAG,
                    ])
                    .status()
                    .await;

                let (mut ready, mut containers) =
                    futures::join!(self.ready.write(), self.containers.write());

                containers.insert(id, Container { name, directory });
                ready.push_back(id);

                status
            };
            commands.push(task);
        }
        let status = join_all(commands).await;
        for cmd in status {
            if cmd.is_err() || !cmd.unwrap().success() {
                eprintln!("Failed to startup new container!");
            }
        }
    }
}
