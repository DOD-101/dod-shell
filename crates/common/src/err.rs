use thiserror::Error;

/// Custom Errors for the shell
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("no default audio card")]
    NoDefaultCard,
    #[error("failed to init WaylandInterface: {0}")]
    WaylandInterfaceFailedInit(String),
}
