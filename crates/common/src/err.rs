//! See [`enum@Error`]
use thiserror::Error;

/// Custom errors for the shell
#[derive(Debug, Error, Clone)]
#[non_exhaustive]
#[allow(
    missing_docs,
    reason = "The error macro gives a good description already"
)]
pub enum Error {
    #[error("no default audio card")]
    NoDefaultCard,
    #[error("failed to init WaylandInterface: {0}")]
    WaylandInterfaceFailedInit(String),
    #[error("failed getting the requested osk layout")]
    MissingOskLayout,
}
