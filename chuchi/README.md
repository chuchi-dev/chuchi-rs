A simple http server library.

## Basic get request

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

	server.add_resource(GlobalName("chuchi".into()));
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


## Api Example

```rust no_run
#[cfg(feature = "api")]
{
use std::fmt;
use std::sync::{Arc, Mutex};

use chuchi::api::{Request, Method};
use chuchi::api::error::{self, Error as ApiError, StatusCode};
use chuchi::{api, impl_res_extractor, Resource};

use serde::{Serialize, Deserialize};


// -- Type definitions

#[derive(Debug, Clone, Serialize)]
pub enum Error {
	Internal(String),
	Request(String)
}

impl error::ApiError for Error {
	fn from_error(e: ApiError) -> Self {
		match e {
			ApiError::HeadersMissing(_) |
			ApiError::Deserialize(_) => {
				Self::Request(e.to_string())
			}
			e => Self::Internal(e.to_string()),
		}
	}

	fn status_code(&self) -> StatusCode {
		match self {
			Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
			Self::Request(_) => StatusCode::BAD_REQUEST
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NameReq;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Name {
	firstname: String,
	lastname: String
}

impl Request for NameReq {
	type Response = Name;
	type Error = Error;

	const PATH: &'static str = "/name";
	const METHOD: Method = Method::GET;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SetNameReq {
	name: Name
}

impl Request for SetNameReq {
	type Response = ();
	type Error = Error;

	const PATH: &'static str = "/name";
	const METHOD: Method = Method::PUT;
}

// -- implementations

#[derive(Resource)]
struct SharedName(Mutex<Name>);

#[api(NameReq)]
fn get_name(req: NameReq, name: &SharedName) -> Result<Name, Error> {
	let lock = name.0.lock().unwrap();
	Ok(lock.clone())
}

#[api(SetNameReq)]
fn set_name(req: SetNameReq, name: &SharedName) -> Result<(), Error> {
	let mut lock = name.0.lock().unwrap();
	*lock = req.name;

	Ok(())
}

#[tokio::main]
async fn main() {
	let name = SharedName(Mutex::new(Name {
		firstname: "Albert".into(),
		lastname: "Einstein".into()
	}));

	let mut server = chuchi::build("0.0.0.0:3000").await
		.expect("Failed to parse address");

	server.add_resource(name);

	server.add_route(get_name);
	server.add_route(set_name);

	server.run().await.unwrap();
}
}
```
