/// TODO: GIVE BETTER DOC STRING
use crate::imports::*;

/// Contains all the page titles, and wraps around indivisual <PageTitle />
/// components
#[component]
pub fn TitleContainer(
	/// Additional class names to apply to the outer div, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Children of the component
	children: Children,
) -> impl IntoView {
	let class = move || format!("p-xxs flex justify-start items-center {}", class.get());

	view! { <div class={class}>{children()}</div> }
}

/// Sets the Text Size of the Page Title
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PageTitleVariant {
	/// Large text
	#[default]
	Heading,
	/// Medium Text
	SubHeading,
	/// Small Text
	Text,
}

/// Specifies where to put the arrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PageTitleIconPosition {
	/// Don't Put an arrow
	#[default]
	None,
	/// Put the arrow in the end, implying that there are more breadcrumbs to
	/// follow
	End,
	/// Put the arrow in the start, implying that this is the last breadcumb
	Start,
}

#[component]
/// Specifies each individual page title,
pub fn PageTitle(
	/// Specifies where to put the arrow.
	#[prop(into, optional)]
	icon_position: MaybeSignal<PageTitleIconPosition>,
	/// Additional class names to pass to the link component, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Children of the component
	children: ChildrenFn,
	/// Title Text Style
	#[prop(into, optional)]
	variant: MaybeSignal<PageTitleVariant>,
	/// The Page to navigate to
	#[prop(into, optional)]
	to: MaybeSignal<String>,
) -> impl IntoView {
	let class = format!(
		"p-xxs fr-fs-ct {} {}",
		match variant.get() {
			PageTitleVariant::Heading => "text-xl",
			PageTitleVariant::SubHeading => "text-md text-white",
			PageTitleVariant::Text => "text-sm text-white",
		},
		class.get()
	);
	let start_icon = move || {
		(icon_position.get() == PageTitleIconPosition::Start).then(|| {
			view! { <Icon icon={IconType::ChevronRight}/> }
		})
	};
	let end_icon = move || {
		(icon_position.get() == PageTitleIconPosition::End).then(|| {
			view! { <Icon size={Size::Small} icon={IconType::ChevronRight} class="mx-xs"/> }
		})
	};

	view! {
		<>
			{start_icon} <Link to={to.get()} r#type={Variant::Link} class={class}>
				{children()}
			</Link> {end_icon}
		</>
	}
}
