use chuchi::{Request, Result};

use serde::{Deserialize, Serialize};

#[macro_use]
mod util;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonData {
	number: u32,
	yes: bool,
	comment: String,
}

#[tokio::test]
async fn read_json() {
	const COMMENT: &str = "Hello, World!";

	// build route
	#[chuchi::post("/")]
	async fn read_json(req: &mut Request) -> Result<String> {
		let data: JsonData = req.deserialize().await?;
		Ok(data.comment)
	}

	let addr = spawn_server!(|builder| {
		builder.add_route(read_json);
	});

	// now do a request
	let correct_req_body = format!(
		"{{\"number\":10,\"yes\":false\
		,\"comment\":\"{}\"}}",
		COMMENT
	);
	make_request!("POST", addr, "/", correct_req_body.clone())
		.await
		// content-type not defined in request
		.assert_status(400);

	make_request!("POST", addr, "/", |builder| {
		builder
			.header("content-type", "application/json")
			.body(chuchi::Body::into_http_body(correct_req_body.into()))
			.expect("request could not be built")
	})
	.await
	.assert_status(200)
	.assert_header("content-type", "text/plain; charset=utf-8")
	.assert_header("content-length", COMMENT.len().to_string())
	.assert_body_str(COMMENT)
	.await;
}

#[tokio::test]
async fn write_json() {
	// build route
	#[chuchi::get_json("/")]
	fn write_json() -> JsonData {
		JsonData {
			number: 10,
			yes: false,
			comment: "Hello, World!".into(),
		}
	}

	let addr = spawn_server!(|builder| {
		builder.add_route(write_json);
	});

	let body = "{\"number\":10,\"yes\":false,\"comment\":\"Hello, World!\"}";
	make_request!("GET", addr, "/")
		.await
		.assert_status(200)
		.assert_header("content-type", "application/json; charset=utf-8")
		.assert_header("content-length", body.len().to_string())
		.assert_body_str(body)
		.await;
}
