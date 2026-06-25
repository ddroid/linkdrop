use thiserror::Error;

#[derive(Debug, Error)]
pub enum LinkdropError {
    #[error("slug already exists: {0}")]
    SlugExists(String),

    #[error("page not found: {0}")]
    NotFound(String),

    #[error("invalid slug: {0}")]
    InvalidSlug(String),

    #[error("content exceeds maximum size of {max} bytes")]
    TooLarge { max: usize },

    #[error("page expired: {0}")]
    Expired(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl LinkdropError {
    pub fn status_code(&self) -> u16 {
        match self {
            LinkdropError::SlugExists(_) => 409,
            LinkdropError::NotFound(_) => 404,
            LinkdropError::InvalidSlug(_) => 400,
            LinkdropError::TooLarge { .. } => 413,
            LinkdropError::Expired(_) => 404,
            LinkdropError::Other(_) => 500,
        }
    }
}
