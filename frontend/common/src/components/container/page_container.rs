use crate::prelude::*;

/// A Single Page container, typically will wrap around all pages
#[component]
pub fn PageContainer(
	/// Additional Class Names to apply to the outer div, if any
	#[props(into, optional)]
	class: String,
	/// The contents of the page
	children: Element,
) -> Element {
	rsx! {
		div { class: "flex items-start justify-start bg-page-container w-full h-full bg-secondary {class}",
			main { class: "flex flex-col items-center justify-center w-full px-lg h-full",
				{children}
			}
		}
	}
}
