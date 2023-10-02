use std::fmt::Display;

/// The routes that the app can take
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppRoute {
	/// The empty route. Mostly used for mounting other components. Must never
	/// be used as a route.
	#[default]
	Empty,
	/// The routes that can be taken when the user is logged in
	LoggedInRoute(LoggedInRoute),
	/// The routes that can be taken when the user is logged out
	LoggedOutRoute(LoggedOutRoute),
}

impl AppRoute {
	/// Returns true if the route is empty, and false otherwise.
	pub fn is_empty(&self) -> bool {
		matches!(self, Self::Empty)
	}
}

/// The routes that can be taken when the user is logged in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedOutRoute {
	/// The login page
	Login,
	/// The sign up page
	SignUp,
	/// The forgot password page
	ForgotPassword,
}

/// The routes that can be taken when the user is logged out
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedInRoute {
	/// The home page
	Home,
}

impl Display for AppRoute {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Empty => write!(f, ""),
			Self::LoggedInRoute(logged_in_routes) => {
				write!(f, "{}", logged_in_routes)
			}
			Self::LoggedOutRoute(logged_out_routes) => {
				write!(f, "{}", logged_out_routes)
			}
		}
	}
}

impl Display for LoggedInRoute {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Home => "/",
			}
		)
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
			}
		)
	}
}
