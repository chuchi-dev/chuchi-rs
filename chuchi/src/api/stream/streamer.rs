use super::error::StreamError;
use super::message::MessageData;

use crate::{
	extractor::Extractor, extractor_extract, extractor_prepare,
	extractor_validate,
};
use std::{convert::Infallible, marker::PhantomData};
use tokio::sync::mpsc;

use serde::{de::DeserializeOwned, Serialize};

pub struct RawStreamer {
	inner: InnerRawStreamer,
}

enum InnerRawStreamer {
	Sender(mpsc::Sender<MessageData>),
	Receiver(mpsc::Receiver<MessageData>),
}

impl RawStreamer {
	pub(crate) fn sender(tx: mpsc::Sender<MessageData>) -> Self {
		Self {
			inner: InnerRawStreamer::Sender(tx),
		}
	}

	pub(crate) fn receiver(rx: mpsc::Receiver<MessageData>) -> Self {
		Self {
			inner: InnerRawStreamer::Receiver(rx),
		}
	}

	pub fn assign_message<M>(self) -> Streamer<M> {
		Streamer {
			inner: self.inner,
			message: PhantomData,
		}
	}
}

pub struct Streamer<M> {
	inner: InnerRawStreamer,
	message: PhantomData<M>,
}

impl<M> Streamer<M> {
	/// ## Panics
	/// If you call this when the Stream::KIND is not Receiver
	pub async fn send(&mut self, data: M) -> Result<(), StreamError>
	where
		M: Serialize,
	{
		match &mut self.inner {
			InnerRawStreamer::Sender(tx) => tx
				.send(MessageData::serialize(data).map_err(StreamError::Json)?)
				.await
				.map_err(|_| StreamError::Closed),
			_ => panic!("Streamer: cannot send, in receive mode"),
		}
	}

	/// Completes when the receiver has dropped.
	///
	/// ## Panics
	/// If you call this when the Stream::KIND is not Receiver
	pub async fn closed(&self) {
		match &self.inner {
			InnerRawStreamer::Sender(tx) => {
				tx.closed().await;
			}
			_ => panic!("Streamer: cannot send, in receive mode"),
		}
	}

	/// ## Panics
	/// If you call this when the Stream::KIND is not Sender
	pub async fn recv(&mut self) -> Result<M, StreamError>
	where
		M: DeserializeOwned,
	{
		match &mut self.inner {
			InnerRawStreamer::Receiver(rx) => {
				let data = rx.recv().await.ok_or(StreamError::Closed)?;

				data.deserialize().map_err(StreamError::Json)
			}
			_ => panic!("Streamer: cannot receive, in sender mode"),
		}
	}
}

impl<'a, R, M> Extractor<'a, R> for Streamer<M>
where
	M: Send + 'static,
{
	type Error = Infallible;
	type Prepared = Self;

	extractor_validate!(|validate| {
		assert!(
			validate.state.validate::<Self>(),
			"Stream not found in state"
		);
		// remove it from the state since we will use it
		validate.state.remove::<Self>();
	});

	extractor_prepare!(|prepare| { Ok(prepare.state.remove().unwrap()) });

	extractor_extract!(|extract| { Ok(extract.prepared) });
}
