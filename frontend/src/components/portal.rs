use leptos_declarative::prelude::*;

use crate::prelude::*;

/// The portal with a wrapper around creating easy portals
#[component]
pub fn Portal(
	/// The scope of the component
	cx: Scope,
	/// The children to put into the portal
	children: ChildrenFn,
) -> impl IntoView {
	view! { cx,
		<PortalInput id={PortalId}>
			{children(cx)}
		</PortalInput>
	}
}
