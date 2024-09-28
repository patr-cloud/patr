use std::rc::Rc;

use ev::MouseEvent;

use crate::prelude::*;

#[component]
pub fn UrlDashboardHead(
	/// On Click Create Managed Url
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_click_create: Callback<MouseEvent>,
) -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="flex justify-between items-center w-full">
				<div class="flex flex-col items-start justify-start">
					<PageTitleContainer>
						<PageTitle icon_position={PageTitleIconPosition::End}>
							"Incfrastructure"
						</PageTitle>
						<PageTitle variant={PageTitleVariant::SubHeading}>"Managed URL"</PageTitle>
					</PageTitleContainer>

					<PageDescription
						description="Create and Manage URLs with ease using Patr."
						doc_link={Some("https://docs.patr.cloud/features/managed-urls/".to_owned())}
					/>
				</div>

				<Link
					r#type={Variant::Button}
					style_variant={LinkStyleVariant::Contained}
					on_click={Rc::new(move |ev| { on_click_create.call(ev.clone()) })}
				>
					"CREATE MANAGED URL"
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
