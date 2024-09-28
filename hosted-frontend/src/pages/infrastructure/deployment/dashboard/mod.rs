mod head;

use convert_case::*;
use leptos_query::QueryResult;

pub use self::head::*;
use super::components::*;
use crate::{
	prelude::*,
	queries::{list_deployments_query, AllDeploymentsTag},
};

/// The Shell Outer for Deployment Page
#[component]
pub fn DeploymentPage() -> impl IntoView {
	view! {
		<ContainerMain class="w-full h-full my-md">
			<Outlet />
		</ContainerMain>
	}
}

/// The Deployment Dashboard Page
#[component]
pub fn DeploymentDashboard() -> impl IntoView {
	let QueryResult {
		data: deployment_list,
		..
	} = list_deployments_query().use_query(move || AllDeploymentsTag);

	view! {
		<DeploymentDashboardHead />

		<ContainerBody>
			<Transition fallback={move || {
				view! { <p>"loading"</p> }
			}}>
				{move || match deployment_list.get() {
					Some(Ok(data)) => {
						view! {
							<section class="p-xl w-full overflow-y-auto">
								<div class="grid gap-lg justify-start content-start
								grid-cols-[repeat(auto-fit,_minmax(300px,_400px))]">
									<For
										each={move || data.deployments.clone()}
										key={|state| state.id}
										let:child
									>
										<DeploymentCard deployment={child} />
									</For>
								</div>
							</section>
						}
							.into_view()
					}
					Some(Err(err)) => {
						view! {
							<ErrorPage
								title="Error Loading Deployments"
								content={view! {
									<p class="text-white">
										{format!("{}", err.to_string().to_case(Case::Title))}
									</p>
								}
									.into_view()}
							/>
						}
							.into_view()
					}
					_ => view! {}.into_view(),
				}}
			</Transition>
		</ContainerBody>
	}
}
