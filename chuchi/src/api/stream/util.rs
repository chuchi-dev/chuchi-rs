use tracing::info;

use crate::extractor::ExtractorError;

use crate::api::error::Error;
use crate::api::ApiError;

use super::error::UnrecoverableError;
use super::message::MessageData;
use super::streamer::RawStreamer;
use super::{Stream, Streamer};

pub fn deserialize_req<S: Stream>(
	msg: MessageData,
) -> Result<S, UnrecoverableError> {
	msg.deserialize()
		.map_err(|e| format!("failed to deserialize stream request {e}").into())
}

#[inline]
pub fn transform_streamer<S: Stream>(
	streamer: RawStreamer,
) -> Streamer<S::Message> {
	streamer.assign_message()
}

pub fn extraction_error<S: Stream>(e: impl ExtractorError) -> S::Error {
	S::Error::from_error(Error::ExtractionError(e.into_std()))
}

pub fn error_to_data<S: Stream>(
	r: Result<(), S::Error>,
) -> Result<MessageData, UnrecoverableError> {
	match r {
		Ok(_) => Ok(MessageData::null()),
		Err(e) => {
			info!(error = ?e, "stream response error");
			// try to convert the error into a message
			MessageData::serialize(e).map_err(|e| e.to_string().into())
		}
	}
}
