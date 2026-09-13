use derive_new::new;
use schemars::JsonSchema;
use serde::Serialize;

use crate::scraper::url::DeniedUrlError;

// Model
#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct OpengraphTitle(pub Option<String>);
#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct OpengraphDescription(pub Option<String>);
#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct OpengraphImage(pub Option<String>);

#[derive(Debug, PartialEq, Eq, new)]
pub struct Opengraph {
    pub title: OpengraphTitle,
    pub description: OpengraphDescription,
    pub image: OpengraphImage,
}

// Error
#[derive(Debug)]
pub enum ScrapeError {
    Denied(DeniedUrlError),
    Upstream,
    UnsupportedMedia,
    TooLarge,
    Timeout,
    Unexpected,
}

impl From<DeniedUrlError> for ScrapeError {
    fn from(error: DeniedUrlError) -> Self {
        ScrapeError::Denied(error)
    }
}
