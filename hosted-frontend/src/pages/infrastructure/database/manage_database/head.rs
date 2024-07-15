use crate::prelude::*;

#[component]
pub fn ManageDatabaseHeader() -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="fr-sb-ct full-width">
				<div class="fc-fs-fs">
					<PageTitleContainer>
						<PageTitle icon_position={PageTitleIconPosition::End}>
							"Infrastructure"
						</PageTitle>
						<PageTitle
							icon_position={PageTitleIconPosition::End}
							variant={PageTitleVariant::SubHeading}
						>
							"Database"
						</PageTitle>
						<PageTitle variant={PageTitleVariant::Text}>"{database_name}"</PageTitle>
					</PageTitleContainer>
				</div>

				<Link
					r#type={Variant::Button}
					style_variant={LinkStyleVariant::Contained}
					color={Color::Error}
					class="fr-ct-ct gap-xs"
				>
					<Icon
						icon={IconType::Trash2}
						size={Size::ExtraSmall}
						color={Color::White}
					/>
					"DELETE"
				</Link>
			</div>
		</ContainerHead>
	}
}
