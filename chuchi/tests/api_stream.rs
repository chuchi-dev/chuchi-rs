use chuchi::api::stream::message::{Message, MessageData, MessageKind};
use chuchi::api::stream::{Stream, StreamKind, StreamServer, Streamer};
use chuchi::body::BodyHttp;
use chuchi::ws::{CloseCode, WebSocket};
use chuchi::{api_stream, Body};

use chuchi::api::error::{self, Error as ApiError, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};

use tokio::time::sleep;
use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::WebSocketStream;

use std::time::Duration;
use std::{fmt, io};

use serde::{Deserialize, Serialize};

use tracing_test::traced_test;

#[derive(Debug, Serialize, Deserialize)]
pub enum Error {
	Internal(String),
	Request(String),
}

impl error::ApiError for Error {
	fn from_error(e: ApiError) -> Self {
		match e {
			ApiError::HeadersMissing(_) | ApiError::Deserialize(_) => {
				Self::Request(e.to_string())
			}
			e => Self::Internal(e.to_string()),
		}
	}

	fn status_code(&self) -> StatusCode {
		match self {
			Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
			Self::Request(_) => StatusCode::BAD_REQUEST,
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PingReq {
	pub name: String,
	pub repeat: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pong {
	pub name: String,
}

impl Stream for PingReq {
	type Message = Pong;
	type Error = Error;

	const KIND: StreamKind = StreamKind::Receiver;
	const ACTION: &'static str = "ping";
}

#[api_stream(PingReq)]
async fn ping_ping(
	req: PingReq,
	mut streamer: Streamer<Pong>,
) -> Result<(), Error> {
	for _ in 0..req.repeat {
		streamer
			.send(Pong {
				name: req.name.clone(),
			})
			.await
			.map_err(|e| Error::Internal(e.to_string()))?;

		sleep(Duration::from_millis(100)).await;
	}

	Ok(())
}

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

macro_rules! ws_client {
	($srv_addr:expr, $uri:expr, |$ws:ident| $block:block) => {
		let addr = $srv_addr.to_string();
		let uri = format!("http://{addr}{}", $uri);

		let req = hyper::Request::builder()
			.uri(uri)
			.header("host", &addr)
			.header("upgrade", "websocket")
			.header("sec-websocket-version", "13")
			.header("sec-websocket-key", "123")
			.body(Body::new().into_http_body())
			.unwrap();

		let resp = send_request(req).await.unwrap();

		assert_eq!(
			resp.status().as_u16(),
			101,
			"didn't receive switching protocols"
		);
		let upgrade_header = resp
			.headers()
			.get("upgrade")
			.map(|v| v.to_str().ok())
			.flatten();
		assert_eq!(
			upgrade_header,
			Some("websocket"),
			"header: upgrade != \"websocket\""
		);

		let upgraded = hyper::upgrade::on(resp)
			.await
			.expect("could not upgrade connection");

		let mut $ws = WebSocket::from_raw(
			WebSocketStream::from_raw_socket(
				TokioIo::new(upgraded),
				Role::Client,
				None,
			)
			.await,
		);
		let _ = async move { $block }.await;
	};
}

#[tokio::test]
#[traced_test]
async fn test_ping() {
	// builder server
	let addr = spawn_server!(|builder| {
		let mut stream_server = StreamServer::new("/ws");
		stream_server.insert(ping_ping);

		builder.add_raw_route(stream_server);
	});

	// make request
	ws_client!(addr, "/ws", |ws| {
		let msg = Message {
			kind: MessageKind::ReceiverRequest,
			action: "ping".into(),
			data: MessageData::serialize(&PingReq {
				name: "ping".into(),
				repeat: 2,
			})
			.unwrap(),
		};

		ws.serialize(&msg).await.expect("could not serialize");

		let msg: Message = ws
			.deserialize()
			.await
			.expect("could not receive")
			.expect("no message received");

		assert_eq!(msg.kind, MessageKind::ReceiverRequest);
		assert_eq!(msg.action, "ping");

		for _ in 0..2 {
			let msg: Message = ws
				.deserialize()
				.await
				.expect("could not receive")
				.expect("no message received");

			// todo there is probably a bug that some messages are not
			// sent if the sender closes to fast
			eprintln!("resv msg: {:?}", msg);

			let resp: Pong =
				msg.data.deserialize().expect("could not deserialize");
			assert_eq!(resp.name, "ping");
		}

		let msg: Message = ws
			.deserialize()
			.await
			.expect("could not receive")
			.expect("no message received");

		assert_eq!(msg.kind, MessageKind::ReceiverClose);
		assert_eq!(msg.action, "ping");

		ws.close(CloseCode::Normal, "".into()).await;
	});

	// close the connection properly
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}
