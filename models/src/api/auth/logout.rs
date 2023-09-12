use crate::prelude::*;

macros::declare_api_endpoint!(
	// Logout
	Logout,
	POST "/auth/sign-out",
);
