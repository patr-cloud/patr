use crate::prelude::*;

macros::declare_api_endpoint!(
	// Definition of a route to logout a user and end the current user session by discarding the
	// authentication token and refresh token. This will no longer provide access to PATR services
	// and the user will have to login again to start a new session.
	Logout,
	POST "/auth/sign-out",
);
