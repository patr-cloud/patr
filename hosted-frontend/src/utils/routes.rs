use std::fmt::Display;

use strum::EnumIter;

/// The list of all the routes on the frontend
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum AppRoutes {
	/// The Empty Route, Used for fallback routes.
	#[default]
	Empty,
	/// The routes that can be taken when the user is logged out.
	LoggedOutRoute(LoggedOutRoute),
	/// The routes that can be taken when the user is logged in.
	LoggedInRoute(LoggedInRoute),
}

#[test]
fn test() {
	use strum::IntoEnumIterator;

	for route in AppRoutes::iter()
		.filter(|route| *route == AppRoutes::Empty)
		.chain(LoggedOutRoute::iter().map(AppRoutes::LoggedOutRoute))
		.chain(LoggedInRoute::iter().map(AppRoutes::LoggedInRoute))
	{
		println!("{:?}", route);
	}
}

/// Logged In Routes, The routes that can be accessed by the user if and only if
/// the user is logged in
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Default)]
pub enum LoggedInRoute {
	/// The Home page
	#[default]
	Home,
	/// The Profile Page
	UserProfile,
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
	/// Workspace Page
	Workspace,
	/// Runners Page
	Runners,
}

/// Logged Out Routes, the routes that can be accessed by the user if and only
/// if the user is logged out
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, Default)]
pub enum LoggedOutRoute {
	/// Login Page
	#[default]
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
				Self::UserProfile => "/user",
				Self::ApiTokens => "/api-tokens",
				Self::Domain => "/domain",
				Self::ManagedUrl => "/managed-url",
				Self::Database => "/database",
				Self::Deployment => "/deployment",
				Self::StaticSites => "/static-site",
				Self::Secret => "/secret",
				Self::ContainerRegistry => "/container-registry",
				Self::Workspace => "/workspace",
				Self::Runners => "/runners",
			}
		)
	}
}
