//! Types related to the `ContentType` http header.
//!
//! ## Note
//! These are the most basic types, types that can be utf8 are always utf8
//! Todo this should be redone once a clear successor of mime is available.

use super::HeaderValue;

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mime(MimeValue);

impl Mime {
	/// Create a Mime Type from a file extension.
	pub fn from_extension(ext: &str) -> Option<Self> {
		MimeValue::from_extension(ext).map(Self)
	}

	pub fn extension(&self) -> &'static str {
		self.0.extension()
	}

	pub fn as_str(&self) -> &'static str {
		self.0.as_str()
	}

	pub fn as_str_with_maybe_charset(&self) -> &'static str {
		self.0.as_str_with_maybe_charset()
	}
}

impl fmt::Display for Mime {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

include!(concat!(env!("OUT_DIR"), "/mime.rs"));

impl FromStr for Mime {
	type Err = ();

	fn from_str(s: &str) -> Result<Self, ()> {
		MimeValue::from_str(s).map(Self).ok_or(())
	}
}

/// Http `ContentType` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
	None,
	Known(Mime),
	Unknown(String),
}

impl ContentType {
	pub fn as_str(&self) -> &str {
		match self {
			Self::Known(m) => m.0.as_str_with_maybe_charset(),
			Self::Unknown(s) => s,
			Self::None => "",
		}
	}

	pub fn from_extension(e: &str) -> Option<Self> {
		Some(Self::Known(Mime::from_extension(e)?))
	}
}

impl fmt::Display for ContentType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl From<()> for ContentType {
	fn from(_: ()) -> Self {
		Self::None
	}
}

impl From<Mime> for ContentType {
	fn from(m: Mime) -> Self {
		Self::Known(m)
	}
}

impl From<String> for ContentType {
	fn from(s: String) -> Self {
		match Mime::from_str(&s) {
			Ok(m) => Self::Known(m),
			Err(_) => Self::Unknown(s),
		}
	}
}

impl<'a> From<&'a str> for ContentType {
	fn from(s: &'a str) -> Self {
		match Mime::from_str(s) {
			Ok(m) => Self::Known(m),
			Err(_) => Self::Unknown(s.to_string()),
		}
	}
}

impl TryFrom<ContentType> for HeaderValue {
	type Error = super::values::InvalidHeaderValue;

	fn try_from(ct: ContentType) -> Result<Self, Self::Error> {
		match ct {
			ContentType::None => Ok(Self::from_static("")),
			ContentType::Known(m) => {
				Ok(Self::from_static(m.as_str_with_maybe_charset()))
			}
			ContentType::Unknown(s) => s.try_into(),
		}
	}
}
