use crate::prelude::*;

#[component]
pub fn CreateRepository() -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="fr-sb-ct full-width">
				<div class="fc-fs-fs">
					<PageTitleContainer>
						<PageTitle icon_position=PageTitleIconPosition::End>
							"Container Registry"
						</PageTitle>
						<PageTitle variant=PageTitleVariant::SubHeading>
							"Container Registry"
						</PageTitle>
					</PageTitleContainer>

					<PageDescription
						description="Create a new Repository here."
						doc_link=Some("https://docs.patr.cloud/features/container-registry/#creating-a-repository".to_owned())
					/>
				</div>
			</div>
		</ContainerHead>
		<ContainerBody class="px-xxl pt-xl pb-sm gap-md">
			<form class="full-width px-md mb-lg full-height fc-fs-fs fit-wide-screen mx-auto">
				<div class="flex full-width">
					<label
						html_for="repo-name"
						class="txt-white txt-sm flex-col-2 fr-fs-fs mt-sm"
					>
						"Repository Name"
					</label>
					<div class="flex-col-10 fc-fs-fs gap-xs">
						<Input
							r#type=InputType::Text
							placeholder="Enter Name"
							class="full-width"
						/>
					</div>
				</div>

				<div class="fr-fs-ct mt-auto ml-auto">
					<Link
						class="mr-xs btn"
					>
						"BACK"
					</Link>
					<Link
						style_variant=LinkStyleVariant::Contained
						should_submit=true
					>
						"CREATE"
					</Link>
				</div>
			</form>
		</ContainerBody>
	}
}
