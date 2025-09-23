
/// The Runner Dashboard Page
mod dashboard;

pub use self::dashboard::*;
use crate::prelude::*;
#[component]
pub fn RunnerPage(children: Children) -> impl IntoView {
	view! {
		<ContainerMain class="w-full h-full my-md">
			{children()}
		</ContainerMain>
	}
}
