use std::net::SocketAddr;

use super::LogWebSocketReturn;
use crate::error::ClientErrorKind;
use crate::extractor::ExtractorError;
use crate::header::{
	RequestHeader, StatusCode, CONNECTION, SEC_WEBSOCKET_ACCEPT,
	SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_VERSION, UPGRADE,
};
use crate::server::HyperRequest;
use crate::util::convert_hyper_req_to_fire_header;
use crate::{Error, Response, Result};

use tracing::error;

use sha1::Digest;

use hyper::upgrade::OnUpgrade;

#[doc(hidden)]
pub use tokio::task::spawn;

use base64::prelude::{Engine as _, BASE64_STANDARD};

/// we need to expose this instead of inlining it in the macro since
/// tracing logs the crate name and we wan't it to be associated with
/// fire http instead of the crate that uses the macro
#[doc(hidden)]
pub fn upgrade_error(e: hyper::Error) {
	error!("websocket upgrade error {:?}", e);
}

/// we need to expose this instead of inlining it in the macro since
/// tracing logs the crate name and we wan't it to be associated with
/// fire http instead of the crate that uses the macro
#[doc(hidden)]
pub fn log_websocket_return(r: impl LogWebSocketReturn) {
	if r.should_log_error() {
		error!("websocket connection closed with error {:?}", r);
	}
}

/// we need to expose this instead of inlining it in the macro since
/// tracing logs the crate name and we wan't it to be associated with
/// fire http instead of the crate that uses the macro
#[doc(hidden)]
pub fn log_extractor_error(r: impl ExtractorError) {
	let err = r.into_std();

	error!("websocket extractor error: {}", err);
}

// does the key need to be a specific length?
#[doc(hidden)]
pub fn upgrade(req: &mut HyperRequest) -> Result<(OnUpgrade, String)> {
	// if headers not match for websocket
	// return bad request
	let header_upgrade =
		req.headers().get(UPGRADE).and_then(|v| v.to_str().ok());
	let header_version = req
		.headers()
		.get(SEC_WEBSOCKET_VERSION)
		.and_then(|v| v.to_str().ok());
	let websocket_key =
		req.headers().get(SEC_WEBSOCKET_KEY).map(|v| v.as_bytes());

	if !matches!(
		(header_upgrade, header_version, websocket_key),
		(Some("websocket"), Some("13"), Some(_))
	) {
		return Err(ClientErrorKind::BadRequest.into());
	}

	// calculate websocket key stuff
	// unwrap does not fail because we check above
	let websocket_key = websocket_key.unwrap();
	let ws_accept = {
		let mut sha1 = sha1::Sha1::new();
		sha1.update(websocket_key);
		sha1.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
		// cannot fail because
		BASE64_STANDARD.encode(sha1.finalize())
	};

	let on_upgrade = hyper::upgrade::on(req);

	Ok((on_upgrade, ws_accept))
}

#[doc(hidden)]
pub fn switching_protocols(ws_accept: String) -> Response {
	Response::builder()
		.status_code(StatusCode::SWITCHING_PROTOCOLS)
		.header(CONNECTION, "upgrade")
		.header(UPGRADE, "websocket")
		.header(SEC_WEBSOCKET_ACCEPT, ws_accept)
		.build()
}

#[doc(hidden)]
pub fn hyper_req_to_header(
	req: &mut HyperRequest,
	address: SocketAddr,
) -> Result<RequestHeader> {
	convert_hyper_req_to_fire_header(req, address).map_err(|e| {
		Error::new(
			ClientErrorKind::BadRequest,
			format!("failed to convert hyper request {:?}", e),
		)
	})
}
