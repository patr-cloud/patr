mod head;
mod runner_card;

pub use self::{head::*, runner_card::*};
use crate::{prelude::*, queries::*};

/// The Runner Dashboard page
#[component]
pub fn RunnerDashboard() -> impl IntoView {
	let runners_list = list_runners_query();

	view! {
		<RunnerDashboardHead />
		<ContainerBody class="p-xs gap-md">
			<DashboardContainer
				gap={Size::Large}
				render_items={view! {
					<Transition>
						{move || match runners_list.get() {
							Some(Ok(data)) => {
								view! {
									<For
										each={move || data.runners.clone()}
										key={|state| state.id}
										let:runner
									>
										<RunnerCard runner={runner} />
									</For>
								}
									.into_any()
							}
							Some(Err(_)) => ().into_any(),
							None => view! { <RunnerCardSkeleton /> }.into_any(),
						}}
					</Transition>
				}
					.into_view()}
			/>
		</ContainerBody>
	}
}
