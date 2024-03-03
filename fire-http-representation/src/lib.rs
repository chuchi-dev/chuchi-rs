#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

/// Reexport the http crate
pub use http;

pub mod body;
pub mod header;
pub use body::Body;

pub mod request;
pub use request::Request;

pub mod response;
pub use response::Response;
