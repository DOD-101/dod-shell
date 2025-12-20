use thiserror::Error;

/// Custom Errors for the shell
#[derive(Debug, Error, Clone)]
#[non_exhaustive]
pub enum Error {
    #[error("no default audio card")]
    NoDefaultCard,
    #[error("failed to init WaylandInterface: {0}")]
    WaylandInterfaceFailedInit(String),
    #[error("failed getting the requested osk layout")]
    MissingOskLayout,
}
