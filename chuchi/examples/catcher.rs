use chuchi::header::{RequestHeader, ResponseHeader, StatusCode};
use chuchi::resources::Resources;
use chuchi::routes::Catcher;
use chuchi::util::PinnedFuture;
use chuchi::{get, Request, Response, Result};

#[get("/")]
fn hello_world() -> &'static str {
	"Hello, World!"
}

struct Error404Handler;

impl Catcher for Error404Handler {
	fn check(&self, _req: &RequestHeader, res: &ResponseHeader) -> bool {
		res.status_code() == &StatusCode::NOT_FOUND
	}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		resp: &'a mut Response,
		_data: &'a Resources,
	) -> PinnedFuture<'a, Result<()>> {
		PinnedFuture::new(async move {
			let path = req.header().uri().path();
			let method = req.header().method();
			resp.body = format!(
				"Error 404: Page \"{}\" With Method \"{}\" Not Found",
				path, method
			)
			.into();

			Ok(())
		})
	}
}

#[tokio::main]
async fn main() {
	let mut server = chuchi::build("0.0.0.0:3000")
		.await
		.expect("Address could not be parsed");

	server.add_route(hello_world);
	server.add_catcher(Error404Handler);

	server.run().await.unwrap();
}
