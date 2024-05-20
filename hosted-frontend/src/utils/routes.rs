use std::fmt::Display;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AppRoutes {
	/// The Empty Route, Used for fallback routes.
	#[default]
	Empty,
	/// The routes that can be taken when the user is logged out.
	LoggedOutRoute(LoggedOutRoute),
	/// The routes that can be taken when the user is logged in.
	LoggedInRoute(LoggedInRoute),
}

/// Logged In Routes, The routes that can be accessed by the user if and only if
/// the user is logged in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedInRoute {
	/// The Home page
	Home,
	/// The Profile Page
	Profile,
	/// The Api Tokens Page
	ApiTokens,
	/// Managed URLs Page
	ManagedUrl,
	/// The Domains Page
	Domain,
	/// Container Registry Page
	ContainerRegistry,
	/// Database Page
	Database,
	/// Delpoyment Page
	Deployment,
	/// Secrets Page
	Secret,
	/// Static Sites Page
	StaticSites,
}

/// Logged Out Routes, the routes that can be accessed by the user if and only
/// if the user is logged out
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedOutRoute {
	/// Login Page
	Login,
	/// Sign Up Page
	SignUp,
	/// Confirm OTP Page
	ConfirmOtp,
	/// Forgot Password Page
	ForgotPassword,
}

impl Display for AppRoutes {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Empty => write!(f, "/"),
			Self::LoggedInRoute(logged_in_routes) => {
				write!(f, "{}", logged_in_routes)
			}
			Self::LoggedOutRoute(logged_out_routes) => {
				write!(f, "{}", logged_out_routes)
			}
		}
	}
}

impl Display for LoggedOutRoute {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Login => "/login",
				Self::SignUp => "/sign-up",
				Self::ForgotPassword => "/forgot-password",
				Self::ConfirmOtp => "/confirm",
			}
		)
	}
}

impl Display for LoggedInRoute {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Home => "/",
				Self::Profile => "/profile",
				Self::ApiTokens => "/api-tokens",
				Self::Domain => "/domain",
				Self::ManagedUrl => "/managed-url",
				Self::Database => "/database",
				Self::Deployment => "/deployment",
				Self::StaticSites => "/static-site",
				Self::Secret => "/secret",
				Self::ContainerRegistry => "/container-registry",
			}
		)
	}
}
