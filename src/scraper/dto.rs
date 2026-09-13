use garde::Validate;
use o2o::o2o;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    core::response::DataResponse,
    scraper::domain::{Opengraph, OpengraphDescription, OpengraphImage, OpengraphTitle},
};

// Request
#[derive(Debug, PartialEq, Eq, Deserialize, JsonSchema, Validate)]
pub struct OpengraphBody {
    #[garde(url)]
    pub url: String,
}

// Response
#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema, o2o)]
#[from_owned(Opengraph)]
pub struct OpengraphData {
    pub title: OpengraphTitle,
    pub description: OpengraphDescription,
    pub image: OpengraphImage,
}

pub type OpengraphResponse = DataResponse<OpengraphData>;
