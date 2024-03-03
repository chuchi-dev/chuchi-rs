use fire::get_json;
use fire_http as fire;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct MyType {
	crazy: &'static str,
	good: &'static str,
}

#[get_json("/")]
fn hello_world() -> MyType {
	MyType {
		crazy: "crazy",
		good: "good",
	}
}

#[tokio::main]
async fn main() {
	let mut server = fire::build("0.0.0.0:3000")
		.await
		.expect("Address could not be parsed");

	server.add_route(hello_world);

	server.ignite().await.unwrap();
}
