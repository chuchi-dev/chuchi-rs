use std::{
	any::{Any, TypeId},
	cell::RefCell,
	collections::{HashMap, HashSet},
	mem::ManuallyDrop,
};

pub struct State {
	inner: HashMap<TypeId, Box<dyn Any + Send>>,
}

impl State {
	pub fn new() -> Self {
		Self {
			inner: HashMap::new(),
		}
	}

	pub fn insert<T>(&mut self, data: T)
	where
		T: Any + Send,
	{
		self.inner.insert(TypeId::of::<T>(), Box::new(data));
	}

	pub fn contains<T>(&self) -> bool
	where
		T: Any + Send,
	{
		self.inner.contains_key(&TypeId::of::<T>())
	}

	pub fn get<T>(&self) -> Option<&T>
	where
		T: Any + Send,
	{
		self.inner
			.get(&TypeId::of::<T>())
			.map(|b| b.downcast_ref::<T>().unwrap())
	}

	pub fn remove<T>(&mut self) -> Option<T>
	where
		T: Any + Send,
	{
		self.inner
			.remove(&TypeId::of::<T>())
			.map(|b| *b.downcast::<T>().unwrap())
	}
}

impl Default for State {
	fn default() -> Self {
		Self::new()
	}
}

pub struct StateValidation {
	inner: HashSet<TypeId>,
}

impl StateValidation {
	pub fn new() -> Self {
		Self {
			inner: HashSet::new(),
		}
	}

	pub fn insert<T>(&mut self)
	where
		T: Any + Send,
	{
		self.inner.insert(TypeId::of::<T>());
	}

	pub fn remove<T>(&mut self)
	where
		T: Any + Send,
	{
		self.inner.remove(&TypeId::of::<T>());
	}

	pub fn validate<T>(&self) -> bool
	where
		T: Any + Send,
	{
		self.inner.contains(&TypeId::of::<T>())
	}
}

impl Default for StateValidation {
	fn default() -> Self {
		Self::new()
	}
}

/// A mutable memory location with dynamically checked borrow rules
///
/// This is similar to `RefCell` but instead of returned wrapped
/// reference, it returns the value directly.
///
/// This means that once the value is borrowed to read it will never
/// get available again.
#[derive(Debug)]
pub struct StateRefCell<T> {
	inner: RefCell<Option<T>>,
}

impl<T> StateRefCell<T> {
	pub const fn new(val: T) -> Self {
		Self {
			inner: RefCell::new(Some(val)),
		}
	}

	/// ## Panics
	/// Panics if the value is currently borrowed.
	pub fn replace(&self, val: T) -> Option<T> {
		self.inner.replace(Some(val))
	}

	/// ## Panics
	/// Panics if the value is currently mutably borrowed.
	pub fn get(&self) -> &T {
		self.try_get().expect("already mutably borrowed")
	}

	/// Tries to get the value if it is not currently borrowed.
	pub fn try_get(&self) -> Option<&T> {
		let r = self.inner.try_borrow().ok()?;

		if r.is_none() {
			return None;
		}

		let r = ManuallyDrop::new(r);
		// since the borrow counter does not get decreased because of the
		// ManuallyDrop and the lifetime not getting expanded this is safe
		let v = unsafe { &*(&**r as *const Option<T>) }.as_ref().unwrap();

		Some(v)
	}

	/// ## Panics
	/// Panics if the value is currently borrowed.
	#[allow(clippy::mut_from_ref)]
	pub fn get_mut(&self) -> &mut T {
		let r = self.inner.borrow_mut();
		let mut r = ManuallyDrop::new(r);
		// since the borrow counter does not get decreased because of the
		// ManuallyDrop and the lifetime not getting expanded this is safe
		unsafe { &mut *(&mut **r as *mut Option<T>) }
			.as_mut()
			.expect("already borrowed")
	}

	/// ## Panics
	/// Panics if the value is currently borrowed.
	pub fn take(&mut self) -> T {
		self.inner.replace(None).expect("already borrowed")
	}

	pub fn into_inner(self) -> T {
		self.inner.into_inner().expect("already borrowed")
	}
}
