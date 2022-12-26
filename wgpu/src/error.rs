use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create a window")]
    Window(#[from] winit::error::OsError),
    #[error("Failed to find a suitable adapter")]
    NoSuitableAdapter,
    #[error("Failed to request a device from the adapter")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("Failed to perform surface request")]
    SurfaceError(#[from] wgpu::SurfaceError),
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
