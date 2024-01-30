/// TODO: GIVE BETTER DOC STRING
use crate::imports::*;

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
	#[default]
	None,
	End,
	Start,
}

/// Specifies each indivisual page title,
#[component]
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
) -> impl IntoView {
	let class = format!(
		"p-xxs fr-fs-ct {} {}",
		match variant.get() {
			PageTitleVariant::Heading => "txt-xl",
			PageTitleVariant::SubHeading => "txt-md txt-white",
			PageTitleVariant::Text => "txt-sm txt-white",
		},
		class.get()
	);
	let start_icon = move || {
		(icon_position.get() == PageTitleIconPosition::Start).then(|| {
			view! {
				<Icon icon=IconType::ChevronRight />
			}
		})
	};
	let end_icon = move || {
		(icon_position.get() == PageTitleIconPosition::End).then(|| {
			view! {
				<Icon size=Size::Small icon=IconType::ChevronRight class="mx-xs" />
			}
		})
	};

	view! {
		<>
			{start_icon}
			<Link class=class>
				{children()}
			</Link>
			{end_icon}
		</>
	}
}
