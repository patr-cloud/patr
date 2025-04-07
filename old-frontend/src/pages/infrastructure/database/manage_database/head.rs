use crate::prelude::*;

#[component]
pub fn ManageDatabaseHeader(
	/// The name of the database
	#[prop(into, optional)]
	name: MaybeSignal<String>,
	/// The ID of the database
	#[prop(into, optional, default = None.into())]
	#[allow(unused)]
	id: MaybeSignal<Option<Uuid>>,
) -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="fr-sb-ct full-width">
				<div class="fc-fs-fs">
					<TitleContainer>
						<PageTitle icon_position={PageTitleIconPosition::End}>
							"Infrastructure"
						</PageTitle>
						<PageTitle
							to="/database"
							icon_position={PageTitleIconPosition::End}
							variant={PageTitleVariant::SubHeading}
						>
							"Database"
						</PageTitle>
						{
							let name = name.get().clone();
							if !name.is_empty() {
								view! {
									<PageTitle variant={PageTitleVariant::Text}>
										{name.clone()}
									</PageTitle>
								}
									.into_view()
							} else {
								view! {}.into_view()
							}
						}
					</TitleContainer>
				</div>

				<Link
					r#type={Variant::Button}
					style_variant={LinkStyleVariant::Contained}
					color={Color::Error}
					class="fr-ct-ct gap-xs"
				>
					<Icon icon={IconType::Trash2} size={Size::ExtraSmall} color={Color::White} />
					"DELETE"
				</Link>
			</div>
		</ContainerHead>
	}
}
