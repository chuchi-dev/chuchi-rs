use crate::header::{HeaderValues, RequestHeader, Uri, HOST};

use std::net::SocketAddr;

use hyper::http::uri::{Authority, Scheme};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HeaderError {
	#[error("No host header found")]
	NoHost,

	#[error("Host header is invalid")]
	HostInvalid,

	#[error("Invalid uri")]
	Uri,
}

type Result<T> = std::result::Result<T, HeaderError>;

pub fn convert_hyper_parts_to_fire_header(
	parts: hyper::http::request::Parts,
	address: SocketAddr,
) -> Result<RequestHeader> {
	let values = HeaderValues::from_inner(parts.headers);
	let uri = fill_uri(parts.uri, &values)?;

	Ok(RequestHeader {
		address,
		method: parts.method,
		uri,
		values,
	})
}

pub fn convert_hyper_req_to_fire_header<B>(
	req: &hyper::Request<B>,
	address: SocketAddr,
) -> Result<RequestHeader> {
	let values = HeaderValues::from_inner(req.headers().clone());
	let uri = fill_uri(req.uri().clone(), &values)?;

	Ok(RequestHeader {
		address,
		method: req.method().clone(),
		uri,
		values,
	})
}

fn fill_uri(uri: Uri, headers: &HeaderValues) -> Result<Uri> {
	let mut parts = uri.into_parts();
	// Try to detect it
	// todo how would it be possible to detect http or https
	parts.scheme = Some(Scheme::HTTP);

	// get host infos
	let host = headers.get(HOST).ok_or(HeaderError::NoHost)?;

	parts.authority = Some(
		Authority::try_from(host.as_bytes())
			.map_err(|_| HeaderError::HostInvalid)?,
	);

	Uri::from_parts(parts).map_err(|_| HeaderError::Uri)
}
