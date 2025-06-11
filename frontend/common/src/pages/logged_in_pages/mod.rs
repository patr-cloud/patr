use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Routable)]
pub enum LoggedInRoutes {
	#[route("/", NotFoundPage)]
	Home,
}
