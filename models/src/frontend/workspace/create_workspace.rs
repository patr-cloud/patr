use crate::prelude::*;

macros::declare_app_route! {
	/// The route that shows the workspace creation form
	CreateWorkspace,
	"/workspace/new",
	requires_login = true,
}
