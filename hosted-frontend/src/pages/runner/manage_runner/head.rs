use codee::string::FromToStringCodec;
use models::api::workspace::runner::Runner;

use crate::prelude::*;

#[component]
pub fn RunnerManageHead(
	/// Runner Info
	#[prop(into)]
	runner_info: MaybeSignal<WithId<Runner>>,
) -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let delete_runner_action = create_server_action::<DeleteRunnerFn>();

	view! {
		<ContainerHead>
			<div class="w-full flex justify-between items-center">
				<div class="flex flex-col items-start justify-between">
					<PageTitleContainer clone:runner_info>
						<PageTitle icon_position={PageTitleIconPosition::End}>
							"CI/CD"
						</PageTitle>
						<PageTitle
							to="/runners"
							variant={PageTitleVariant::SubHeading}
							icon_position={PageTitleIconPosition::End}
						>
							"Runners"
						</PageTitle>
						<PageTitle
							variant={PageTitleVariant::SubHeading}
						>
							{
								let runner_info = runner_info.clone();
								move || runner_info.get().name.clone()
							}
						</PageTitle>
					</PageTitleContainer>

					<PageDescription
						description={"Create and manage CI Runners for automated builds.".to_string()}
						doc_link={Some("https://docs.patr.cloud/ci-cd/#choosing-a-runner".to_string())}
					/>
				</div>

				<ActionForm action={delete_runner_action}>
					<input
						type="hidden"
						id="access_token"
						name="access_token"
						value={move || access_token.get()}
					/>

					<input
						type="hidden"
						id="workspace_id"
						name="workspace_id"
						value={move || workspace_id.get()}
					/>

					<input
						type="hidden"
						id="runner_id"
						name="runner_id"
						value={
							let runner_info = runner_info.clone();
							move || runner_info.get().id.to_string()
						}
					/>

					<Link
						style_variant={LinkStyleVariant::Contained}
						should_submit={true}
						class="text-white btn-error"
					>
						<Icon
							icon=IconType::Trash2
							size=Size::ExtraSmall
							color=Color::White
							class="mr-xs"
						/>
						"DELETE"
					</Link>
				</ActionForm>
			</div>
		</ContainerHead>
	}
}
