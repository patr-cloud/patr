#[macro_export]
macro_rules! query (
	($query:literal) => ({
		let mut logged_query = format!("{}", $query).replace("\n", " ").replace("\t", " ");
		while logged_query.contains("  ") {
			logged_query = logged_query.replace("  ", " ");
		}
		logged_query = logged_query.trim().to_string();
		log::info!(target: "api::queries", "{}", logged_query);
		sqlx::query!($query)
	});
	($query:literal, $($args:expr),*$(,)?) => ({
		let mut logged_query = format!("{}", $query).replace("\n", " ").replace("\t", " ");
		while logged_query.contains("  ") {
			logged_query = logged_query.replace("  ", " ");
		}
		logged_query = logged_query.trim().to_string();
		log::info!(target: "api::queries", "{}", logged_query);
		sqlx::query!($query, $($args), *)
	});
);

#[macro_export]
macro_rules! query_as (
	($ty_as:ident, $query:literal) => ({
		let mut logged_query = format!("{}", $query).replace("\n", " ").replace("\t", " ");
		while logged_query.contains("  ") {
			logged_query = logged_query.replace("  ", " ");
		}
		logged_query = logged_query.trim().to_string();
		log::info!(target: "api::queries", "{}", logged_query);
		sqlx::query_as!($ty_as, $query)
	});
	($ty_as:ident, $query:literal, $($args:expr),*$(,)?) => ({
		let mut logged_query = format!("{}", $query).replace("\n", " ").replace("\t", " ");
		while logged_query.contains("  ") {
			logged_query = logged_query.replace("  ", " ");
		}
		logged_query = logged_query.trim().to_string();
		log::info!(target: "api::queries", "{}", logged_query);
		sqlx::query_as!($ty_as, $query, $($args), *)
	});
);

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
