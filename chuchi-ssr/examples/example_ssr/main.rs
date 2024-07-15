use serde::{Serialize, Deserialize};

use fire_ssr::JsServer;

use fire::{get, get_json, Request, Response, Error};
use fire::fs::StaticFiles;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct News {
	title: String,
	content: String
}

#[get_json("/api/news")]
fn test_api() -> Vec<News> {
	vec![
		News {
			title: "Sommarugas abrupter Rücktritt ist verständlich, kommt aber \
			zur Unzeit".into(),
			content: "Die Schweiz befindet sich mitten in der grössten \
			Energiekrise seit Jahrzehnten. Nun verliert das Land ausgerechnet \
			jene Bundesrätin, die im Krisenmanagement die Fäden in den Händen \
			hält.".into()
		}
	]
}

#[get("/*")]
async fn all(
	req: &mut Request,
	ssr: &JsServer
) -> Result<Response, Error> {
	ssr.request(req).await
		.map_err(Error::from_server_error)
}


#[tokio::main]
async fn main() {
	tracing_subscriber::fmt()
		.with_env_filter("error,fire_http=trace")
		.init();

	let mut fire = fire::build("0.0.0.0:3000").await.unwrap();

	fire.add_route(test_api);
	fire.add_route(StaticFiles::new("/assets", "./../example-ssr/dist/assets"));

	let js_server = JsServer::new(
		"./examples/example_ssr/public/js",
		include_str!("./public/index.html"),
		(),
		// 2 cores
		2
	);

	fire.add_data(js_server);
	fire.add_route(all);

	fire.ignite().await.unwrap();
}