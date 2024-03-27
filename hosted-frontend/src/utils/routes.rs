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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedInRoute {
	Home,
	Profile,
	ApiTokens,
	ManagedUrl,
	Domain,
	ContainerRegistry,
	Database,
	Deployment,
	Secret,
	StaticSites,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedOutRoute {
	Login,
	SignUp,
	ConfirmOtp,
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
