use chuchi::extractor::PathParam;
use chuchi::ws::{CloseCode, Error, WebSocket};
use chuchi::Body;
use chuchi::{impl_res_extractor, ws};

use tokio_tungstenite::tungstenite::protocol::Role;
use tokio_tungstenite::WebSocketStream;

use hyper_util::rt::TokioIo;
use tracing_test::traced_test;

#[macro_use]
mod util;

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

		let resp = util::send_request(req).await.unwrap();

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
async fn build_con() {
	// TODO improve test
	// to look if we get connection closed properly
	// handle errors in task

	// // build route
	#[ws("/")]
	async fn websocket_route(
		mut ws: WebSocket,
		_: &SomeData,
		_: &SomeData,
	) -> Result<(), Error> {
		let mut c = 0;

		while let Some(msg) = ws.receive().await? {
			// read
			assert_eq!(msg.to_text().unwrap(), format!("Hey {}", c));
			c += 1;
			// send them
			ws.send("Hi").await?;
		}

		println!("connection closed");

		Ok(())
	}

	#[derive(Debug)]
	struct SomeData;

	impl_res_extractor!(SomeData);

	// builder server
	let addr = spawn_server!(|builder| {
		builder.add_resource(SomeData);
		builder.add_raw_route(websocket_route);
	});

	// make request
	ws_client!(addr, "/", |ws| {
		for i in 0..5 {
			ws.send(format!("Hey {}", i)).await.expect("could not send");
			let msg = ws
				.receive()
				.await
				.expect("could not receive")
				.expect("no message received");
			assert_eq!(msg.to_text().expect("not text"), "Hi");
		}
		ws.close(CloseCode::Normal, "".into()).await;
	});

	// close the connection properly
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}

#[tokio::test]
#[traced_test]
async fn ws_params() {
	// TODO improve test
	// to look if we get connection closed properly
	// handle errors in task

	// // build route
	#[ws("/{id}")]
	async fn websocket_route(
		mut ws: WebSocket,
		id: PathParam<usize>,
		_: &SomeData,
		_: &SomeData,
	) -> Result<(), Error> {
		let mut c = 0;

		while let Some(msg) = ws.receive().await? {
			// read
			assert_eq!(msg.to_text().unwrap(), format!("Hey {}", c));
			c += 1;
			// send them
			ws.send(format!("Hi {id}")).await?;
		}

		println!("connection closed");

		Ok(())
	}

	#[derive(Debug)]
	struct SomeData;

	impl_res_extractor!(SomeData);

	// builder server
	let addr = spawn_server!(|builder| {
		builder.add_resource(SomeData);
		builder.add_raw_route(websocket_route);
	});

	// make request
	ws_client!(addr, "/42", |ws| {
		for i in 0..5 {
			ws.send(format!("Hey {}", i)).await.expect("could not send");
			let msg = ws
				.receive()
				.await
				.expect("could not receive")
				.expect("no message received");
			assert_eq!(msg.to_text().expect("not text"), "Hi 42");
		}
		ws.close(CloseCode::Normal, "".into()).await;
	});

	// close the connection properly
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}
