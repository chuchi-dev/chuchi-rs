
use fire_http as fire;

use fire::{Request, Response, Body, Data, get, post};
use fire::header::{RequestHeader, ResponseHeader, StatusCode, Mime};
use fire::routes::Catcher;
use fire::util::PinnedFuture;

#[macro_use]
mod util;


#[tokio::test]
async fn hello_world() {
	const BODY: &str = "Hello, World!";

	// build route
	#[get("/")]
	fn hello_world() -> &'static str { BODY }

	let addr = spawn_server!(|builder| {
		builder.add_route(hello_world);
	});

	// now do a request
	make_request!("GET", addr, "/").await
		.assert_status(200)
		.assert_header("content-type", "text/plain; charset=utf-8")
		.assert_header("content-length", BODY.len().to_string())
		.assert_body_str(BODY).await;
}

#[tokio::test]
async fn test_post() {
	const BODY: &str = "Hello, World!";

	// build route
	#[post("/")]
	fn post(req: &mut Request) -> Body {
		req.take_body()
	}

	let addr = spawn_server!(|builder| {
		builder.add_route(post);
	});

	// now do a request
	make_request!("POST", addr, "/", BODY).await
		.assert_status(200)
		// this is the default content-type
		// we should probably change that
		.assert_not_header("content-type")
		// because we return a stream
		// we don't know how big it is
		.assert_not_header("content-length")
		.assert_body_str(BODY).await;
}

#[tokio::test]
async fn test_catcher() {
	const BODY: &str = "Body not Found";

	struct NotFound;

	impl Catcher for NotFound {
		fn check(&self, _req: &RequestHeader, res: &ResponseHeader) -> bool {
			res.status_code() == &StatusCode::NOT_FOUND
		}

		fn call<'a>(
			&'a self,
			_req: Request,
			res: Response,
			_data: &'a Data
		) -> PinnedFuture<'a, fire::Result<Response>> {
			PinnedFuture::new(async move {
				let resp = Response::builder()
					.status_code(*res.header().status_code())
					.content_type(Mime::TEXT)
					.body(BODY)
					.build();

				Ok(resp)
			})
		}
	}

	let addr = spawn_server!(|builder| {
		builder.add_catcher(NotFound);
	});

	// now do a request
	make_request!("GET", addr, "/").await
		.assert_status(404)
		// this is the default content-type
		// we should probably change that
		.assert_header("content-type", "text/plain; charset=utf-8")
		.assert_header("content-length", BODY.len().to_string())
		.assert_body_str(BODY).await;
}

#[tokio::test]
async fn anything() {
	struct Data(Vec<u8>);

	// some random data
	let mut data = vec![];
	for i in 1..=1024 {
		data.push((i % 255) as u8);
	}

	// build route
	#[get("/")]
	fn get(data: &Data) -> Vec<u8> {
		data.0.clone()
	}

	let addr = spawn_server!(
		|builder| {
			builder.add_data(Data(data.clone()));
			builder.add_route(get);
		}
	);

	// now do a request
	make_request!("GET", addr, "/").await
		.assert_status(200)
		// this is the default content-type
		// we should probably change that
		.assert_header("content-type", "application/octet-stream")
		.assert_header("content-length", data.len().to_string())
		.assert_body_vec(&data).await;
}