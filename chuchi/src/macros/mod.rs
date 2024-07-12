#[macro_export]
macro_rules! impl_res_extractor {
	($ty:ty) => {
		impl<'a, R> $crate::extractor::Extractor<'a, R> for &'a $ty {
			type Error = std::convert::Infallible;
			type Prepared = ();

			$crate::extractor_validate!(|validate| {
				assert!(
					validate.resources.exists::<$ty>(),
					"Resource {} does not exist",
					stringify!($ty)
				);
			});

			$crate::extractor_prepare!();

			$crate::extractor_extract!(|extract| {
				Ok(extract.resources.get::<$ty>().unwrap())
			});
		}
	};
}

#[macro_export]
macro_rules! impl_req_extractor {
	($ty:ty) => {
		impl<'a> $crate::extractor::Extractor<'a, $ty> for $ty {
			type Error = std::convert::Infallible;
			type Prepared = ();

			$crate::extractor_validate!();

			$crate::extractor_prepare!();

			$crate::extractor_extract!(<$ty> |extract| {
				Ok(extract.request.take().unwrap())
			});
		}
	};
}

#[macro_export]
macro_rules! extractor_validate {
	() => {
		$crate::extractor_validate!(|_validate| {});
	};
	(|$validate:ident| $block:block) => {
		fn validate($validate: $crate::extractor::Validate<'_>) {
			$block
		}
	};
}

#[macro_export]
macro_rules! extractor_prepare {
	() => {
		$crate::extractor_prepare!(|_prepare| { Ok(()) });
	};
	(|$prepare:ident| $block:block) => {
		fn prepare(
			$prepare: $crate::extractor::Prepare<'_>,
		) -> std::pin::Pin<
			std::boxed::Box<
				dyn std::future::Future<
						Output = std::result::Result<
							Self::Prepared,
							Self::Error,
						>,
					> + Send
					+ '_,
			>,
		> {
			Box::pin(async move { $block })
		}
	};
}

#[macro_export]
macro_rules! extractor_extract {
	// () => {
	// 	$crate::extractor_prepare!(|_prepare| { Ok(()) });
	// };
	(|$extract:ident| $block:block) => {
		$crate::extractor_extract!(<R> |$extract| $block);
	};
	(<$r:ty> |$extract:ident| $block:block) => {
		fn extract(
			$extract: $crate::extractor::Extract<'a, '_, Self::Prepared, $r>,
		) -> std::result::Result<Self, Self::Error>
		where
			Self: Sized,
		{
			$block
		}
	};
}
