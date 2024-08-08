use crate::prelude::*;

#[component]
pub fn DatabaseHead() -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="fr-sb-ct full-width">
				<div class="fc-fs-fs">
					<PageTitleContainer>
						<PageTitle icon_position={PageTitleIconPosition::End}>
							"Infrastructure"
						</PageTitle>
						<PageTitle variant={PageTitleVariant::SubHeading}>"Database"</PageTitle>
					</PageTitleContainer>

					<PageDescription
						description="Create and manage Databases using Patr."
						doc_link={Some(
							"https://docs.patr.cloud/features/databases/".to_owned(),
						)}
					/>

				</div>

				<Link
					r#type={Variant::Link}
					to="create"
					style_variant={LinkStyleVariant::Contained}
				>
					"CREATE DATABASE"
					<Icon
						icon={IconType::Plus}
						size={Size::ExtraSmall}
						class="ml-xs"
						color={Color::Black}
					/>
				</Link>
			</div>
		</ContainerHead>
	}
}
