use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppRoute {
	#[default]
	Empty,
	LoggedInRoutes(LoggedInRoutes),
	LoggedOutRoutes(LoggedOutRoutes),
}

impl AppRoute {
	pub fn is_empty(&self) -> bool {
		matches!(self, Self::Empty)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedOutRoutes {
	Login,
	SignUp,
	ForgotPassword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedInRoutes {
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
