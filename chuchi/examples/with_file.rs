use chuchi::fs::StaticFile;

const HELLO_WORLD: StaticFile =
	StaticFile::no_cache("/", "./examples/www/hello_world.html");

#[tokio::main]
async fn main() {
	let mut server = chuchi::build("0.0.0.0:3000")
		.await
		.expect("Address could not be parsed");

	server.add_route(HELLO_WORLD);

	server.run().await.unwrap();
}
