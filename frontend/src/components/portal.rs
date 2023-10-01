#![allow(unused_variables)]

use crate::prelude::*;

/// The portal with a wrapper around creating easy portals
#[component]
pub fn Portal(
	/// The children to put into the portal
	children: ChildrenFn,
) -> impl IntoView {
	view! {
		// <PortalInput id={PortalId}>
		// 	{children()}
		// </PortalInput>
	}
}
