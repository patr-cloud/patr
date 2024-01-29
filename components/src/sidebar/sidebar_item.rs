use crate::imports::*;

#[component]
pub fn SidebarItem(
    #[prop(into)]
    link: MaybeSignal<LinkItem>,
    #[prop(into, optional)]
    class: MaybeSignal<String>,
) -> impl IntoView {
    view! {
        <li class="sidebar-item full-width fc-fs-fs">
            <Link style_variant=LinkStyleVariant::Plain class="btn full-width py-sm">
                <img src={link.get().icon_src} alt={link.get().title} />
                <span class="ml-md txt-md fc-fs-fs txt-left">
                    <span class="pos-rel txt-md txt-left">
                        {link.get().title}
                    </span>
                    <small class="txt-xxs txt-grey">
                        {link.get().subtitle}
                    </small>
                </span>
            </Link>
        </li>
    }
}