mod footer;
mod head;

use convert_case::*;
use leptos_query::QueryResult;

use self::{footer::*, head::*};
use super::{components::*, utils::*};
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
	let deployment_page = create_rw_signal(0);
	create_effect(move |_| {
		use_navigate()(
			format!("/deployment?page={}", deployment_page.get()).as_str(),
			Default::default(),
		);
	});

	let QueryResult {
		data: deployment_list,
		..
	} = list_deployments_query().use_query(move || AllDeploymentsTag(deployment_page.get()));

	let total_count = Signal::derive(move || match deployment_list.get() {
		Some(Ok((count, _))) => count,
		_ => 0,
	});

	view! {
		<DeploymentDashboardHead />

		<ContainerBody>
			<Transition fallback={move || {
				view! { <p>"loading"</p> }
			}}>
				{move || match deployment_list.get() {
					Some(Ok(data)) => {
						view! {
							<ContainerGrid
								min_width={"300px"}
								max_width={"400px"}
							>
								<For
									each={move || data.1.deployments.clone()}
									key={|state| state.id}
									let:child
								>
									<DeploymentCard deployment={child} />
								</For>
							</ContainerGrid>
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
					_ => view! {
							<ContainerGrid
								min_width={"300px"}
								max_width={"400px"}
							>
								<DeploymentSkeletonCard />
							</ContainerGrid>
					}.into_view(),
				}}
			</Transition>

			<DeploymentDashboardFooter
				total_count={total_count}
				current_page={deployment_page}
			/>
		</ContainerBody>
	}
}
