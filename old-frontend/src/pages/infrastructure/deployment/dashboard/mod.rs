mod footer;
mod head;

use convert_case::*;

use self::{footer::*, head::*};
use super::{components::*, utils::*};
use crate::{prelude::*, queries::list_deployments_query};

/// The Shell Outer for Deployment Page
#[component]
pub fn DeploymentPage() -> impl IntoView {
	view! {
		<ContainerMain class="w-full h-full my-md">
			<Outlet />
		</ContainerMain>
	}
}

#[component]
fn LoadingDeployments() -> impl IntoView {
	view! {
		<ContainerGrid
			min_width={"300px"}
			max_width={"400px"}
		>
			<For
				each={move || (0..constants::RESOURCES_PER_PAGE).collect::<Vec<usize>>()}
				key={|state| state.clone()}
				let:_
			>
				<DeploymentSkeletonCard />
			</For>
		</ContainerGrid>
	}
}

/// The Deployment Dashboard Page
#[component]
pub fn DeploymentDashboard() -> impl IntoView {
	let deployment_page = create_rw_signal(0);
	let (state, _) = AuthState::load();

	create_effect(move |_| {
		use_navigate()(
			format!("/deployment?page={}", deployment_page.get()).as_str(),
			Default::default(),
		);
	});

	let deployment_list = list_deployments_query(deployment_page.into());

	let total_count = Signal::derive(move || match deployment_list.get() {
		Some(Ok((count, _))) => count,
		_ => 0,
	});

	view! {
		<DeploymentDashboardHead />

		<ContainerBody>
			<Transition
				fallback={move || {
					view! { <LoadingDeployments /> }
				}}
			>
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
						<LoadingDeployments />
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
