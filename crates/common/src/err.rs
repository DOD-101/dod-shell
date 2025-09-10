use thiserror::Error;

/// Custom Errors for the shell
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("no default audio card")]
    NoDefaultCard,
}
