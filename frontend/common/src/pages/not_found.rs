use crate::prelude::*;

#[component]
pub fn NotFoundPage() -> Element {
	rsx! {
		PageContainer { class: "bg-onboard",
			h1 { "404 - Not Found" }
		}
	}
}
