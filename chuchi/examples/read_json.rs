use chuchi::{get, post, Request, Response, Result};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MyType {
	crazy: String,
	good: String,
}

#[get("/")]
fn hello_world() -> Response {
	Response::html(
		"<a id=\"btn\" style=\"color: blue\">Send MyType</a>
		<script>
			const myType = {
				crazy: 'crazy',
				good: 'good'
			};
			document.getElementById('btn').addEventListener('click', async e => {
				const res = await fetch('/', {
					method: 'POST',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify(myType)
				}).then(r => r.text());
				alert(`response: ${ await res }`);
			});
		</script>",
	)
}

#[post("/")]
async fn hello_world_json(req: &mut Request) -> Result<String> {
	let my_type: MyType = req.deserialize().await?;
	Ok(format!("read type {:?}", my_type))
}

#[tokio::main]
async fn main() {
	let mut server = chuchi::build("0.0.0.0:3000")
		.await
		.expect("Address could not be parsed");

	server.add_route(hello_world);
	server.add_route(hello_world_json);

	server.run().await.unwrap();
}
