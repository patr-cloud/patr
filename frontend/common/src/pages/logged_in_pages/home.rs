use crate::prelude::*;

#[component]
pub fn HomePage() -> Element {
	rsx! {
		PageContainer { class: "bg-onboard" }
	}
}
