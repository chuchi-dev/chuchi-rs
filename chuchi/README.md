A simple http server library.

## Example

```rust no_run
use chuchi::{get, Res};

struct GlobalName(String);

// handle a simple get request
#[get("/")]
fn root(global_name: Res<GlobalName>) -> String {
	format!("Hi, this is {}", global_name.0)
}

#[tokio::main]
async fn main() {
	let mut server = chuchi::build("0.0.0.0:3000").await
		.expect("Failed to parse address");

	server.add_resource(GlobalName("fire".into()));
	server.add_route(root);

	server.run().await.unwrap();
}
```

For more examples look in the examples directory and the test directory.

## Features

-   json
-   fs
-   http2 (enables http 2 support)
-   ws (adds websocket support)
-   trace
