use crate::imports::*;

#[component]
pub fn StaticSiteDashboard() -> impl IntoView {
	let data = create_rw_signal(vec![0, 1, 2]);
	view! {
		<ContainerMain>
			<ContainerHead>
				<div class="fr-sb-ct full-width">
					<div class="fc-fs-fs">
						<PageTitleContainer>
							<PageTitle icon_position=PageTitleIconPosition::End>
								"Infrastructure"
							</PageTitle>
							<PageTitle variant=PageTitleVariant::SubHeading>
								"Static Site"
							</PageTitle>
						</PageTitleContainer>

						<PageDescription
							description="Deploy And Manage Static Sites using Patr"
							doc_link=Some("https://docs.patr.cloud/features/static-sites/".to_owned())
						/>
					</div>

					<Link
						r#type=Variant::Button
						style_variant=LinkStyleVariant::Contained
					>
						"CREATE SECRET"
						<Icon
							icon=IconType::Plus
							size=Size::ExtraSmall
							class="ml-xs"
							color=Color::Black
						/>
					</Link>
				</div>
			</ContainerHead>

			<ContainerBody>
				<DashboardContainer
					gap=Size::Large
					render_items=view! {
						<For
							each=move || data.get()
							key=|state| state.clone()
							let:child
						>
							<div class="fc-fs-fs static-site-card bg-secondary-light br-sm px-xl py-md">
								<div class="fr-sb-ct full-width head pb-sm">
									<div class="fr-ct-ct">
										<h5 class="txt-white txt-md txt-thin mr-sm of-hidden txt-of-ellipsis w-20">
											"Name"
										</h5>

										<StatusBadge status=Status::Live />
									</div>

									<button class="fr-ct-ct">
										<Icon color=Color::Error icon=IconType::PlayCircle size=Size::Medium />
									</button>
								</div>

								<a
									href="https://ca100402d79f408f98c202945cfb0310.onpatr.cloud/"
									target="_blank"
									rel="noreferrer"
									class="br-sm of-hidden mt-md full-width full-height bg-secondary-dark outline-primary-focus pos-rel site-preview-sm"
								>
									<iframe
										width="500"
										height="300"
										class="br-sm of-hidden frame pos-abs"
										src="https://ca100402d79f408f98c202945cfb0310.onpatr.cloud/"
									/>
								</a>

								<div class="fr-sb-ct mt-xs full-width px-xxs">
									<Link class="letter-sp-md txt-sm fr-fs-ct">
										"MANAGE STATIC SITE"
										<Icon
											icon=IconType::ChevronRight
											size=Size::ExtraSmall
											color=Color::Primary
										/>
									</Link>
								</div>
							</div>
						</For>
					}.into_view()
				/>
			</ContainerBody>

		</ContainerMain>
	}
}
