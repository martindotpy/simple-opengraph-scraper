use derive_new::new;
use schemars::JsonSchema;
use serde::Serialize;

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
pub enum DeniedUrl {
    InvalidUrl,
    NotHttp,
    Credentials,
    ForbiddenPort,
    ForbiddenHost,
    Unresolvable,
}

#[derive(Debug)]
pub enum ScrapeError {
    Denied(DeniedUrl),
    Upstream,
    UnsupportedMedia,
    TooLarge,
    Timeout,
    Unexpected,
}

impl From<DeniedUrl> for ScrapeError {
    fn from(error: DeniedUrl) -> Self {
        ScrapeError::Denied(error)
    }
}
