use models::utils::Uuid;

macros::declare_app_route! {
	/// Route for Manage Profile Page
	ManageProfile,
	"/user",
	requires_login = true,
}

macros::declare_app_route! {
	/// Route for Api Tokens Page
	ApiTokens,
	"/user/api-tokens",
	requires_login = true,
}

macros::declare_app_route! {
	/// Route for Create Api Tokens Page
	CreateApiToken,
	"/user/api-tokens/create",
	requires_login = true,
}

macros::declare_app_route! {
	/// Route for Edit Api Tokens Page
	EditApiToken,
	"/user/api-tokens/:token_id" {
		/// The ID of the token to edit
		pub token_id: Uuid,
	},
	requires_login = true,
}
