# Chuchi to Axum Migration Guide

This guide covers the migration from chuchi web framework to axum, focusing on the key changes and edge cases encountered during the migration.

## Dependencies

### Remove chuchi dependencies
```toml
# Remove these from Cargo.toml
chuchi = { version = "0.1.0", features = ["fs", "sentry", "api"] }
byte-parser = "0.2.3"
```

### Add axum dependencies
```toml
# Add these to Cargo.toml
axum = { version = "0.8.4", features = ["macros"] }
tower-http = { version = "0.6.6", features = ["cors", "fs", "trace"] }
tower = { version = "0.5", features = ["util"] }
serde_json = "1.0"
serde_urlencoded = "0.7"
http-body-util = "0.1"

# Add signal feature to tokio if not already present
tokio = { version = "1.0", features = ["macros", "rt-multi-thread", "fs", "signal"] }
```

## Application State

### Before (chuchi)
```rust
let mut server = chuchi::build("0.0.0.0:3000").await.unwrap();
server.add_resource(cfg);
server.add_resource(db);
server.add_resource::<data::Users>(Box::new(users));
```

### After (axum)
```rust
#[derive(Clone)]
struct AppState {
    users: data::Users,
    emails: data::Emails,
    organisations: data::Organisations,
    settings: data::Settings,
    pools: data::Pools,
    beneficiaries: data::Beneficiaries,
    db: Db,
    cfg: Arc<Config>,
    dist_dir: DistDir,
}

let app = Router::new()
    .nest("/api/users", routes::users::routes())
    .with_state(state);
```

## Resource Extractors

### Before (chuchi)
```rust
use chuchi::impl_res_extractor;

pub type Users = Box<dyn UsersBuilderTrait + Send + Sync>;
impl_res_extractor!(Users);
```

### After (axum)
```rust
use axum::extract::FromRef;
use std::sync::Arc;

pub type Users = Arc<Box<dyn UsersBuilderTrait + Send + Sync>>;

impl FromRef<AppState> for Users {
    fn from_ref(state: &AppState) -> Self {
        state.users.clone()
    }
}
```

## Handler Functions

### Before (chuchi)
```rust
use chuchi::api;

#[api(AllReq)]
async fn all(
    req: AllReq,
    header: &RequestHeader,
    conn: ConnOwned,
    users: &Users,
    orgs: &Organisations,
) -> Result<All, Error> {
    // handler logic
    Ok(All { list })
}
```

### After (axum)
```rust
use axum::extract::{Path, Query, State};
use axum::{Json, Router};

async fn all(
    Query(query): Query<AllQuery>,
    headers: HeaderMap,
    conn: ConnOwned,
    State(orgs): State<Organisations>,
    State(users): State<Users>,
) -> Result<Json<All>, Error> {
    // handler logic
    Ok(Json(All { list }))
}
```

### ⚠️ CRITICAL: Body Extractors Must Come Last
**Any extractor that consumes the request body (like `Json`, `Bytes`, `String`) MUST be the last parameter in your handler function.**

```rust
// ❌ WRONG - Json extractor before other extractors
async fn handler(
    Json(req): Json<MyRequest>,
    State(db): State<Database>,
    Path(id): Path<String>,
) -> Result<Json<MyResponse>, Error> {
    // This will fail at runtime!
}

// ✅ CORRECT - Json extractor comes last
async fn handler(
    Path(id): Path<String>,
    State(db): State<Database>,
    Json(req): Json<MyRequest>,  // Body extractor LAST
) -> Result<Json<MyResponse>, Error> {
    // This works!
}
```

## Request/Response Types

### Before (chuchi)
```rust
use chuchi::api::{Method, Request};

impl Request for AllReq {
    type Response = All;
    type Error = Error;
    const PATH: &'static str = "/api/users";
    const METHOD: Method = Method::GET;
    const HEADERS: &'static [&'static str] = &["user-token"];
}
```

### After (axum)
```rust
// Remove Request trait implementations entirely
// Path, method, and headers are now handled by axum routing and extractors

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AllQuery {
    email: Option<EmailAddress>,
}
```

## Error Handling

### Before (chuchi)
```rust
use chuchi::api::error::{self, Error as ApiError, StatusCode};

impl error::ApiError for Error {
    fn from_error(e: ApiError) -> Self {
        // error conversion logic
    }

    fn status_code(&self) -> StatusCode {
        // status code logic
    }
}
```

### After (axum)
```rust
use axum::http::StatusCode;
use axum::{response::{IntoResponse, Response}, Json};

impl Error {
    pub fn status_code(&self) -> StatusCode {
        // status code logic (same as before)
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = self.status_code();
        (status, Json(self)).into_response()
    }
}
```

## Route Registration

### Before (chuchi)
```rust
pub fn routes(server: &mut Chuchi) {
    server.add_route(all);
    server.add_route(one);
    server.add_route(new);
    server.add_route(update);
    server.add_route(delete);
}
```

### After (axum)
```rust
use axum::routing::{get, post, put, delete};

// is nested at /api/organisations
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(all).post(new))
        .route("/{id}", get(one).put(update).delete(delete_org))
}
```

## Path Parameters

### Before (chuchi)
```rust
use chuchi::extractor::PathParam;

async fn one(
    req: OneReq,
    id: PathParam<UniqueId>,
    // ...
) -> Result<OneResp, Error> {
    // use id directly
}
```

### After (axum)
```rust
use axum::extract::Path;

async fn one(
    Path(id): Path<UniqueId>,
    // ...
) -> Result<Json<OneResp>, Error> {
    // use id directly
}
```

## JSON Request Bodies

### Before (chuchi)
```rust
#[api(NewReq)]
async fn new(
    req: NewReq,
    // ...
) -> Result<User, Error> {
    let mut req = req.0; // unwrap the request
    // ...
}
```

### After (axum)
```rust
async fn new(
    Json(req): Json<NewReq>,
    // ...
) -> Result<Json<User>, Error> {
    let req = req.0; // unwrap the request
    // ...
    Ok(Json(user))
}
```

## CORS Handling

### Before (chuchi)
```rust
if args.enable_cors || cfg!(debug_assertions) {
    server.add_catcher(routes::cors::CorsHeaders);
}
```

### After (axum)
```rust
use tower_http::cors::{CorsLayer, Any as cors};
use axum::http::{header, Method};

if args.enable_cors || cfg!(debug_assertions) {
    app = app.layer(
        CorsLayer::new()
            .allow_origin(cors::Any)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
            ])
            .allow_headers([
                header::CONTENT_TYPE,
                "user-token".parse().unwrap(),
                "additional-data".parse().unwrap(),
            ]),
    );
}
```

## Coding Style Conventions

When migrating to axum, follow these coding style guidelines from the project:

### 1. Avoid Top-Level Inline Imports
```rust
// ❌ Avoid
use std::*;
use crate::*;

// ✅ Prefer
use std::collections::HashMap;
use crate::data::users::User;
```
Only use inline imports if there are naming conflicts.

### 2. Prefer Direct Function References Over Closures
```rust
// ❌ Avoid
users
    .by_id(&id)
    .await?
    .ok_or(Error::NotFound)?
    .map(|v| Json(v))

// ✅ Prefer
users
    .by_id(&id)
    .await?
    .ok_or(Error::NotFound)?
    .map(Json)
```

### 3. Use Functional Programming Style
```rust
// ❌ Avoid intermediate variables
let user = users
    .by_id(&id)
    .await
    .map_err(|e| Error::Internal(e.to_string()))?
    .ok_or(Error::NotFound)?;

Ok(Json(user))

// ✅ Prefer chaining operations
users
    .by_id(&id)
    .await
    .map_err(|e| Error::Internal(e.to_string()))?
    .ok_or(Error::NotFound)
    .map(Json)
```

## Server Setup

### Before (chuchi)
```rust
let mut server = chuchi::build("0.0.0.0:3000").await.unwrap();
// add resources and routes
init(&mut server);
server.run().await.unwrap();
```

### After (axum)
```rust
use tokio::net::TcpListener;

let listener = TcpListener::bind("0.0.0.0:3000")
    .await
    .expect("failed to bind to port 3000");

axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
```

## Edge Cases and Important Notes

### 1. Body Extractors Must Come Last (CRITICAL!)
**This is the most common source of runtime errors when migrating to axum.**

Any extractor that consumes the request body (like `Json`, `Bytes`, `String`, `Form`) must be the **last parameter** in your handler function. This is because these extractors consume the entire request body, leaving nothing for subsequent extractors.

```rust
// ❌ WRONG - Json extractor before other extractors
async fn handler(
    Json(req): Json<MyRequest>,        // ← This consumes the body
    State(db): State<Database>,        // ← This will fail
    Path(id): Path<String>,            // ← This will fail
) -> Result<Json<MyResponse>, Error> {
    // Runtime error: "Cannot have two extractors that consume the request body"
}

// ✅ CORRECT - Json extractor comes last
async fn handler(
    Path(id): Path<String>,            // ← Path parameters first
    State(db): State<Database>,        // ← State extractors next
    headers: HeaderMap,                // ← Headers if needed
    Json(req): Json<MyRequest>,        // ← Body extractor LAST
) -> Result<Json<MyResponse>, Error> {
    // This works perfectly!
}
```

**Body-consuming extractors include:**
- `Json<T>`
- `Form<T>`
- `Bytes`
- `String`
- Custom extractors that read the request body

### 2. State Types Must Be Arc-Wrapped
Resources in axum state must be `Arc<Box<dyn Trait>>` instead of `Box<dyn Trait>`:

```rust
// Before
pub type Users = Box<dyn UsersBuilderTrait + Send + Sync>;

// After
pub type Users = Arc<Box<dyn UsersBuilderTrait + Send + Sync>>;
```

### 3. Response Wrapping
All successful responses must be wrapped in `Json()`:

```rust
// Before
Ok(user)

// After
Ok(Json(user))
```

### 4. Custom Response Types
For custom responses (like file downloads), use axum's `Response` type:

```rust
use axum::response::Response;
use axum::body::Body;

Response::builder()
    .header(CONTENT_TYPE, "application/zip")
    .body(Body::from(buf))
    .unwrap()
```

### 5. Header Extraction
Headers are extracted differently:

```rust
// Before
async fn handler(header: &RequestHeader) {
    // access headers
}

// After
use axum::http::HeaderMap;

async fn handler(headers: HeaderMap) {
    // access headers
}
```

### 6. Authentication Middleware
Custom authentication extractors need to be reimplemented using axum's `FromRequestParts`:

```rust
#[async_trait]
impl<S> FromRequestParts<S> for CheckedUser<R, T>
where
    S: Send + Sync,
    Users: FromRef<S>,
{
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        // implement authentication logic
    }
}
```

### 7. Graceful Shutdown
Implement graceful shutdown for production:

```rust
use tokio::signal;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
```

### 8. Query Parameter Parsing
Query parameters are extracted using the `Query` extractor:

```rust
// Before
let req: AllReq = req.deserialize_query()?;

// After
async fn handler(Query(query): Query<AllQuery>) {
    // query is automatically deserialized
}
```

## Migration Checklist

- [ ] Update dependencies in Cargo.toml
- [ ] Create AppState struct
- [ ] Implement FromRef for all resource types
- [ ] Remove Request trait implementations
- [ ] Update handler function signatures
- [ ] Wrap responses in Json()
- [ ] Update error handling with IntoResponse
- [ ] Convert route registration to Router::new()
- [ ] Update CORS handling
- [ ] Ensure body extractors come last
- [ ] Test all endpoints
- [ ] Implement graceful shutdown
- [ ] Update authentication extractors
- [ ] Verify file upload/download endpoints
