#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

#[doc(hidden)]
#[macro_use]
pub mod util;
pub mod error;
mod request;
pub mod response;
#[cfg(feature = "stream")]
#[cfg_attr(docsrs, doc(cfg(feature = "feature")))]
pub mod stream;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use error::ApiError;
pub use request::{Method, Request};

#[doc(hidden)]
pub use fire;
#[doc(hidden)]
pub use serde_json;

pub use codegen::api;
#[cfg(feature = "stream")]
pub use codegen::api_stream;
