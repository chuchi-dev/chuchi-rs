#![allow(dead_code, unused_macros)]

use chuchi::body::BodyHttp;
use chuchi::Body;
use hyper_util::rt::TokioExecutor;

use std::io;

macro_rules! spawn_server {
	(|$builder:ident| $block:block) => {{
		use std::net::{Ipv4Addr, SocketAddr};

		let socket_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0);
		let mut $builder = chuchi::build(socket_addr).await.unwrap();
		let _ = $block;
		let fire = $builder.build().await.unwrap();
		let addr = fire.local_addr().unwrap();
		tokio::task::spawn(fire.run());

		addr
	}};
}

macro_rules! other_err {
	($e:expr) => {
		io::Error::new(io::ErrorKind::Other, $e)
	};
}

#[cfg(feature = "http1")]
pub async fn send_request(
	req: hyper::Request<BodyHttp>,
) -> io::Result<hyper::Response<hyper::body::Incoming>> {
	let client =
		hyper_util::client::legacy::Client::builder(TokioExecutor::new())
			.build_http();
	client
		.request(req.map(Box::pin))
		.await
		.map_err(|e| other_err!(e))
}

#[cfg(not(feature = "http1"))]
pub async fn send_request(
	req: hyper::Request<BodyHttp>,
) -> io::Result<hyper::Response<hyper::body::Incoming>> {
	panic!("http1 feature is required for this test")
}

macro_rules! make_request {
	(
		$method:expr, $srv_addr:expr, $uri:expr,
		|$builder:ident| $block:block
	) => {
		async {
			let addr = $srv_addr.to_string();
			let uri = format!("http://{addr}{}", $uri);
			let $builder = hyper::Request::builder()
				.method($method)
				.uri(uri)
				.header("host", &addr);
			let resp = util::send_request($block)
				.await
				.expect("failed to send request")
				.map(chuchi::Body::from_hyper);

			util::TestResponse::new(resp)
		}
	};
	($method:expr, $srv_addr:expr, $uri:expr, $body:expr) => {
		make_request!($method, $srv_addr, $uri, |builder| {
			builder
				.body(chuchi::Body::into_http_body($body.into()))
				.expect("could not build request")
		})
	};
	($method:expr, $srv_addr:expr, $uri:expr) => {
		make_request!($method, $srv_addr, $uri, chuchi::Body::new())
	};
}

#[derive(Debug)]
pub struct TestResponse {
	inner: hyper::Response<Body>,
}

impl TestResponse {
	pub fn new(inner: hyper::Response<Body>) -> Self {
		Self { inner }
	}

	#[track_caller]
	pub fn assert_status(self, other: u16) -> Self {
		assert_eq!(
			self.inner.status().as_u16(),
			other,
			"status code doens't match"
		);
		self
	}

	#[track_caller]
	pub fn assert_header(self, key: &str, value: impl AsRef<str>) -> Self {
		let v = self
			.inner
			.headers()
			.get(key)
			.unwrap_or_else(|| panic!("header with key {:?} not found", key))
			.to_str()
			.expect("header does not only contain visible ASCII chars");
		assert_eq!(v, value.as_ref(), "value does not match");
		self
	}

	#[track_caller]
	pub fn assert_not_header(self, key: &str) -> Self {
		if self.inner.headers().get(key).is_some() {
			panic!("expected no header named {}", key);
		}
		self
	}

	pub fn header(&self, key: &str) -> Option<&str> {
		self.inner.headers().get(key).and_then(|v| v.to_str().ok())
	}

	pub async fn assert_body_str(mut self, value: &str) -> Self {
		let body = self
			.inner
			.body_mut()
			.take()
			.into_string()
			.await
			.expect("could not convert response body to string");
		assert_eq!(body, value, "body does not match value");
		self
	}

	pub async fn assert_body_vec(mut self, value: &[u8]) -> Self {
		let body = self
			.inner
			.body_mut()
			.take()
			.into_bytes()
			.await
			.expect("could not convert response body to vec");
		assert_eq!(body, value, "body does not match value");
		self
	}
}
