#[macro_export]
macro_rules! pin_fn (
	($fn_name:ident) => ({
		|context, next| {
			Box::pin(async move {
				$fn_name(context, next).await
			})
		}
	});
);
