use std::fmt::Display;

/// The routes that the app can take
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppRoute {
	/// The empty route. Mostly used for mounting other components. Must never
	/// be used as a route.
	#[default]
	Empty,
	/// The routes that can be taken when the user is logged in
	LoggedInRoutes(LoggedInRoutes),
	/// The routes that can be taken when the user is logged out
	LoggedOutRoutes(LoggedOutRoutes),
}

impl AppRoute {
	/// Returns true if the route is empty, and false otherwise.
	pub fn is_empty(&self) -> bool {
		matches!(self, Self::Empty)
	}
}

/// The routes that can be taken when the user is logged in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedOutRoutes {
	/// The login page
	Login,
	/// The sign up page
	SignUp,
	/// The forgot password page
	ForgotPassword,
}

/// The routes that can be taken when the user is logged out
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedInRoutes {
	/// The home page
	Home,
}

impl Display for AppRoute {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Empty => write!(f, ""),
			Self::LoggedInRoutes(logged_in_routes) => {
				write!(f, "{}", logged_in_routes)
			}
			Self::LoggedOutRoutes(logged_out_routes) => {
				write!(f, "{}", logged_out_routes)
			}
		}
	}
}

impl Display for LoggedInRoutes {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Home => "/home",
			}
		)
	}
}

impl Display for LoggedOutRoutes {
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
