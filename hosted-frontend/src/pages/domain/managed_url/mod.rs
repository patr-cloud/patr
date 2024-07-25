mod dashboard;

pub use self::dashboard::*;
use crate::prelude::*;

#[component]
pub fn ManagedUrlPage() -> impl IntoView {
	view! {
		<ContainerMain>
			<Outlet />
		</ContainerMain>
	}
}
