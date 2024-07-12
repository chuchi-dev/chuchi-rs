use super::error::ApiError;

pub use crate::header::Method;

use serde::{de::DeserializeOwned, Serialize};

/// Basic request definition.
///
/// The request will be serialized and deserialized
/// via Json to ease updating structures without breaking backwards
/// compatibility.
pub trait Request: Serialize + DeserializeOwned {
	type Response: Serialize + DeserializeOwned;
	type Error: ApiError;

	const PATH: &'static str;
	const METHOD: Method;
	const SIZE_LIMIT: usize = 4096;
	const TIMEOUT: usize = 60;
	const HEADERS: &'static [&'static str] = &[];
}
