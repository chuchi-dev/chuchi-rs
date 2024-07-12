use chuchi::fs::StaticFiles;
use chuchi::Body;

#[macro_use]
mod util;

#[tokio::test]
async fn read_file() {
	// build route
	const CSS: StaticFiles = StaticFiles::new("/css", "./examples/www/css");

	let addr = spawn_server!(|builder| {
		builder.add_route(CSS);
	});

	make_request!("GET", addr, "/css")
		.await
		// folder should be not found
		.assert_status(404)
		.assert_not_header("cache-control")
		.assert_not_header("content-type")
		.assert_header("content-length", "0");

	let file_ctn = include_str!("./../examples/www/css/style.css");
	let res = make_request!("GET", addr, "/css/style.css")
		.await
		.assert_status(200)
		.assert_header("content-length", file_ctn.len().to_string())
		.assert_header("content-type", "text/css; charset=utf-8")
		// 86400 = 60 * 60 * 24
		.assert_header("cache-control", "max-age=86400, public");

	let etag = res.header("etag").expect("etag not found");

	make_request!("GET", addr, "/css/style.css", |req| {
		req.header("if-none-match", etag)
			.body(Body::new().into_http_body())
			.expect("could not build request")
	})
	.await
	.assert_status(304)
	.assert_header("cache-control", "max-age=86400, public")
	.assert_not_header("content-type");
}
