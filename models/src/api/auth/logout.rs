macros::declare_api_endpoint!(
	Logout,
	POST "/auth/sign-out",
	authenticator = PlainTokenAuthenticator,
);
