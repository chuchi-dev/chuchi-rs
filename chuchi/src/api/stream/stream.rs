//! The names are in the perspective of the client

use super::message::MessageKind;
use crate::api::error::ApiError;

use serde::{de::DeserializeOwned, Serialize};

/// ## Note
/// The names are in the perspective of a client so a StreamKind of Sender will
/// mean the client sends data and the server receives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamKind {
	Sender,
	Receiver,
}

impl StreamKind {
	pub fn into_kind_message(self) -> MessageKind {
		match self {
			Self::Sender => MessageKind::SenderMessage,
			Self::Receiver => MessageKind::ReceiverMessage,
		}
	}
}

impl From<MessageKind> for StreamKind {
	fn from(m: MessageKind) -> Self {
		match m {
			MessageKind::SenderRequest
			| MessageKind::SenderMessage
			| MessageKind::SenderClose => Self::Sender,
			MessageKind::ReceiverRequest
			| MessageKind::ReceiverMessage
			| MessageKind::ReceiverClose => Self::Receiver,
		}
	}
}

/// The struct of the stream itself is like a request to start the stream
pub trait Stream: Serialize + DeserializeOwned {
	type Message: Serialize + DeserializeOwned;
	/// After an error occured the stream get's closed
	type Error: ApiError;

	const KIND: StreamKind;
	const ACTION: &'static str;
}
