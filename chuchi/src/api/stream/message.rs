//! The names are in the perspective of the client

use std::borrow::Cow;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageKind {
	SenderRequest,
	SenderMessage,
	SenderClose,
	ReceiverRequest,
	ReceiverMessage,
	ReceiverClose,
}

impl MessageKind {
	pub fn into_close(self) -> Self {
		match self {
			Self::SenderRequest | Self::SenderMessage | Self::SenderClose => {
				Self::SenderClose
			}
			Self::ReceiverRequest
			| Self::ReceiverMessage
			| Self::ReceiverClose => Self::ReceiverClose,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
	pub kind: MessageKind,
	pub action: Cow<'static, str>,
	#[serde(default = "MessageData::null")]
	pub data: MessageData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageData {
	inner: serde_json::Value,
}

impl MessageData {
	pub fn null() -> Self {
		Self {
			inner: serde_json::Value::Null,
		}
	}

	pub fn serialize<S>(value: S) -> Result<Self, serde_json::Error>
	where
		S: Serialize,
	{
		Ok(Self {
			inner: serde_json::to_value(value)?,
		})
	}

	pub fn deserialize<T>(self) -> Result<T, serde_json::Error>
	where
		T: DeserializeOwned,
	{
		serde_json::from_value(self.inner)
	}
}
