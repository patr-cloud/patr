use crate::prelude::*;

// Sidebar Item
#[component]
pub fn SidebarItem(
	/// The target of the link
    #[prop(into, optional)]
	link: LinkItem,
    /// Disabled
    #[prop(into, optional)]
    disabled: bool,
	/// Class names to appy to the link, if any
    #[prop(optional, into)]
    class: MaybeSignal<String>
) -> impl IntoView {
    view! {
        <li class="full-width sidebar-item fc-fs-fs">
            <ALink
                to={link.path}
                class={"btn full-width py-sm"}
            >
                {link.title}
            </ALink>
        </li>
    }
}