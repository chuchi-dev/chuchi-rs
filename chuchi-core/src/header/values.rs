use std::borrow::Cow;
use std::fmt;

pub use http::header::{
	AsHeaderName, HeaderName, HeaderValue, IntoHeaderName, InvalidHeaderValue,
};

#[cfg(feature = "json")]
pub use serde_json::Error as JsonError;

/// Contains all http header values.
///
/// This is really similar to `http::header::HeaderMap` except
/// that is uses IntoHeaderValue for inserting. And it does not allow
/// multiples values for a given key.
#[derive(Debug, Clone)]
pub struct HeaderValues(http::HeaderMap<HeaderValue>);

impl HeaderValues {
	/// Creates a new empty `HeaderValues`.
	pub fn new() -> Self {
		Self(http::HeaderMap::new())
	}

	/// Creates a new `HeaderValues` from it's inner type.
	pub fn from_inner(inner: http::HeaderMap<HeaderValue>) -> Self {
		Self(inner)
	}

	/// Insert a new key and value into the header.
	///
	/// If a value to this key is already present
	/// that value is dropped.
	///
	/// ## Panics
	/// If the value is not a valid HeaderValue.
	pub fn insert<K, V>(&mut self, key: K, val: V) -> Option<HeaderValue>
	where
		K: IntoHeaderName,
		V: TryInto<HeaderValue>,
		V::Error: fmt::Debug,
	{
		let val = val.try_into().expect("invalid HeaderValue");
		self.0.insert(key, val)
	}

	/// Insert a new key and value into the header. Returning
	/// None if the value is not valid.
	///
	/// If a value to this key is already present
	/// that value is dropped.
	pub fn try_insert<K, V>(
		&mut self,
		key: K,
		val: V,
	) -> Result<Option<HeaderValue>, InvalidHeaderValue>
	where
		K: IntoHeaderName,
		V: TryInto<HeaderValue, Error = InvalidHeaderValue>,
	{
		Ok(self.0.insert(key, val.try_into()?))
	}

	/// Insert a new key and value into the header. Percent encoding
	/// the value if necessary.
	pub fn encode_value<K, V>(&mut self, key: K, val: V) -> Option<HeaderValue>
	where
		K: IntoHeaderName,
		V: IntoEncodedHeaderValue,
	{
		let val = val.into_encoded_header_value();
		self.0.insert(key, val)
	}

	/// Insert a new key and a serializeable value. The value will be serialized
	/// as json and percent encoded.
	///
	/// Returns `None` if the value could not be serialized or inserted.
	#[cfg(feature = "json")]
	#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
	pub fn serialize_value<K, V>(
		&mut self,
		key: K,
		val: &V,
	) -> Result<Option<HeaderValue>, JsonError>
	where
		K: IntoHeaderName,
		V: serde::Serialize + ?Sized,
	{
		let v = serde_json::to_string(val)?;
		Ok(self.encode_value(key, v))
	}

	/// Returns the value if it exists.
	pub fn get<K>(&self, key: K) -> Option<&HeaderValue>
	where
		K: AsHeaderName,
	{
		self.0.get(key)
	}

	/// Returns the value mutably if it exists.
	pub fn get_mut<K>(&mut self, key: K) -> Option<&mut HeaderValue>
	where
		K: AsHeaderName,
	{
		self.0.get_mut(key)
	}

	/// Returns the value as a string if it exists and is valid.
	pub fn get_str<K>(&self, key: K) -> Option<&str>
	where
		K: AsHeaderName,
	{
		self.get(key).and_then(|v| v.to_str().ok())
	}

	/// Returns the value percent decoded as a string if it exists and is valid.
	pub fn decode_value<K>(
		&self,
		key: K,
	) -> Option<Result<Cow<'_, str>, std::str::Utf8Error>>
	where
		K: AsHeaderName,
	{
		self.get(key).map(|v| {
			percent_encoding::percent_decode(v.as_bytes()).decode_utf8()
		})
	}

	/// Deserializes a given value. Returning `None` if the value
	/// does not exist or is not valid json.
	#[cfg(feature = "json")]
	#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
	pub fn deserialize_value<K, D>(
		&self,
		key: K,
	) -> Option<Result<D, serde_json::Error>>
	where
		K: AsHeaderName,
		D: serde::de::DeserializeOwned,
	{
		self.get(key).map(|v| {
			let s = percent_encoding::percent_decode(v.as_bytes())
				.decode_utf8_lossy();
			serde_json::from_str(s.as_ref())
		})
	}

	/// Returns the inner `HeaderMap`.
	pub fn into_inner(self) -> http::HeaderMap<HeaderValue> {
		self.0
	}
}

impl Default for HeaderValues {
	fn default() -> Self {
		Self::new()
	}
}

fn encode_to_header_value(s: impl AsRef<[u8]>) -> HeaderValue {
	let s: String = percent_encoding::percent_encode(
		s.as_ref(),
		percent_encoding::CONTROLS,
	)
	.collect();
	// does not allocate again
	let b: bytes::Bytes = s.into();
	// now lets make a header value
	// TODO probably can be changed to
	// from maybe shared unchecked
	// but need to check first control covers all cases
	HeaderValue::from_maybe_shared(b).unwrap()
}

/// Converts a value into a `HeaderValue` and encodes it if necessary.
pub trait IntoEncodedHeaderValue {
	fn into_encoded_header_value(self) -> HeaderValue;
}

macro_rules! impl_into_header_value {
	($(
		$s:ty, $self:ident => $ex:expr
	),*) => ($(
		impl IntoEncodedHeaderValue for $s {
			#[inline]
			fn into_encoded_header_value($self) -> HeaderValue { $ex }
		}
	)*);
	(REF, $(
		$s:ty, $self:ident => $ex:expr
	),*) => ($(
		impl<'a> IntoEncodedHeaderValue for &'a $s {
			#[inline]
			fn into_encoded_header_value($self) -> HeaderValue { $ex }
		}
	)*);
}

impl_into_header_value! {
	HeaderName, self => self.into(),
	HeaderValue, self => self,
	i16, self => self.into(),
	i32, self => self.into(),
	i64, self => self.into(),
	isize, self => self.into(),
	u16, self => self.into(),
	u32, self => self.into(),
	u64, self => self.into(),
	usize, self => self.into(),
	String, self => encode_to_header_value(self),
	Vec<u8>, self => encode_to_header_value(self)
}

impl_into_header_value! { REF,
	HeaderValue, self => self.clone(),
	[u8], self => encode_to_header_value(self),
	str, self => encode_to_header_value(self)
}

#[cfg(test)]
mod tests {
	#![allow(unused_imports)]

	use super::*;
	use serde::{Deserialize, Serialize};

	#[test]
	fn test_encdec() {
		let mut values = HeaderValues::new();
		values.encode_value("Rocket", "🚀 Rocket");
		let s = values.get_str("Rocket").unwrap();
		assert_eq!(s, "%F0%9F%9A%80 Rocket");

		let s = values.decode_value("Rocket").unwrap().unwrap();
		assert_eq!(s, "🚀 Rocket");
	}

	#[cfg(feature = "json")]
	#[test]
	fn test_serde() {
		#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
		struct Value {
			text: String,
			number: usize,
		}

		let mut values = HeaderValues::new();
		let val = Value {
			text: "🚀 Rocket".into(),
			number: 42,
		};
		values.serialize_value("Value", &val).unwrap();

		let s = values.get_str("Value").unwrap();
		assert_eq!(s, "{\"text\":\"%F0%9F%9A%80 Rocket\",\"number\":42}");

		let n_val: Value = values.deserialize_value("Value").unwrap().unwrap();
		assert_eq!(n_val, val);
	}
}
