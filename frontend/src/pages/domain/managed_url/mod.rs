mod dashboard;

pub use self::dashboard::*;
use crate::prelude::*;

#[component]
pub fn ManagedUrlPage() -> impl IntoView {
	view! {
		<ContainerMain class="my-md">
			<Outlet />
		</ContainerMain>
	}
}
