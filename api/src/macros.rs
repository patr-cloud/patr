#[macro_export]
macro_rules! error (
	($err_name:ident) => ({
		serde_json::json!({
			$crate::utils::constants::request_keys::SUCCESS: false,
			$crate::utils::constants::request_keys::ERROR: $crate::models::error::id::$err_name,
			$crate::utils::constants::request_keys::MESSAGE: $crate::models::error::message::$err_name
		})
	});
);
