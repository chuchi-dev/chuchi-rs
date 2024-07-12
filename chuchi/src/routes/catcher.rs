use crate::header::{RequestHeader, ResponseHeader};
use crate::util::PinnedFuture;
use crate::{Request, Resources, Response};

// Catcher Trait
pub trait Catcher: Send + Sync {
	fn check(&self, req: &RequestHeader, res: &ResponseHeader) -> bool;

	// check if every data you expect is in Data
	fn validate_data(&self, _data: &Resources) {}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		resp: &'a mut Response,
		data: &'a Resources,
	) -> PinnedFuture<'a, crate::Result<()>>;
}
