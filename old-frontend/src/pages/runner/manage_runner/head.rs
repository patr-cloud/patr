use models::api::workspace::runner::Runner;

use crate::{prelude::*, queries::delete_runner_query};

#[component]
pub fn RunnerManageHead(
	/// Runner Info
	#[prop(into)]
	runner_info: MaybeSignal<WithId<Runner>>,
) -> impl IntoView {
	let delete_runner_action = delete_runner_query();

	view! {
		<ContainerHead>
			<div class="w-full flex justify-between items-center">
				<div class="flex flex-col items-start justify-between">
					<TitleContainer clone:runner_info>
						<PageTitle icon_position={PageTitleIconPosition::End}>"CI/CD"</PageTitle>
						<PageTitle
							to="/runners"
							variant={PageTitleVariant::SubHeading}
							icon_position={PageTitleIconPosition::End}
						>
							"Runners"
						</PageTitle>
						<PageTitle variant={PageTitleVariant::SubHeading}>
							{
								let runner_info = runner_info.clone();
								move || runner_info.get().name.clone()
							}
						</PageTitle>
					</TitleContainer>

					<PageDescription
						description={"change me to a description".to_string()}
						doc_link={Some(
							"https://docs.patr.cloud/ci-cd/#choosing-a-runner".to_string(),
						)}
					/>
				</div>

				<form on:submit={move |ev| {
					ev.prevent_default();
					delete_runner_action.dispatch(runner_info.get().id.clone());
				}}>
					<Link
						style_variant={LinkStyleVariant::Contained}
						should_submit=true
						class="text-white btn-error"
					>
						<Icon
							icon={IconType::Trash2}
							size={Size::ExtraSmall}
							color={Color::White}
							class="mr-xs"
						/>
						"DELETE"
					</Link>
				</form>
			</div>
		</ContainerHead>
	}
}
