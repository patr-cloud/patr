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

#[macro_export]
macro_rules! error (
	($err_name:ident) => ({
		serde_json::json!({
			request_keys::SUCCESS: false,
			request_keys::ERROR: error::id::$err_name,
			request_keys::MESSAGE: error::message::$err_name
		})
	});
);
