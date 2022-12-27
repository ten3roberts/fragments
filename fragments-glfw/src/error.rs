use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("No window backend is running")]
    NoBackend,
    #[error("Failed to initialize glfw")]
    GlfwInit(#[from] glfw::InitError),
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
