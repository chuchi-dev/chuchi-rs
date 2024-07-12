use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct Resources {
	inner: Arc<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl Resources {
	pub(crate) fn new() -> Self {
		Self {
			inner: Arc::new(HashMap::new()),
		}
	}

	pub fn exists<R>(&self) -> bool
	where
		R: Any,
	{
		self.inner.contains_key(&TypeId::of::<R>())
	}

	/// returns true if the data already existed
	pub(crate) fn insert<R>(&mut self, data: R) -> bool
	where
		R: Any + Send + Sync,
	{
		let map = Arc::get_mut(&mut self.inner).unwrap();
		map.insert(data.type_id(), Box::new(data)).is_some()
	}

	pub fn get<R>(&self) -> Option<&R>
	where
		R: Any,
	{
		self.inner
			.get(&TypeId::of::<R>())
			.and_then(|a| a.downcast_ref())
	}
}

#[cfg(feature = "graphql")]
impl juniper::Context for Resources {}
