//! Starting a new container every request adds unwanted
//! overhead so we instead keep them alive to be reused

#[derive(Default)]
pub struct DockerPool {
    // pub pool: 
}
