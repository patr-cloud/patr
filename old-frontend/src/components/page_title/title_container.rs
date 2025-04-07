use crate::imports::*;

/// Page Title Item, used to specify the title Segments of the Page Title
/// Mulitple Segements can be used as breadcrumbs.
#[derive(Clone)]
pub struct PageTitleItem {
	/// Title of the Title Segment
	pub title: String,
	/// The Link of the Title Segment
	pub link: Option<String>,
	/// The Arrow Icon Position of the Title Segment
	pub icon_position: PageTitleIconPosition,
	/// The Variant of the Title Segment, Heading, SubHeading, or Text
	pub variant: PageTitleVariant,
}

#[component]
pub fn PageTitleContainer(
	/// Page Title Items
	#[prop(into)]
	page_title_items: MaybeSignal<Vec<PageTitleItem>>,
	/// The Title of the description, Set this to None if no description is
	/// required
	#[prop(into, optional, default = None.into())]
	description_title: MaybeSignal<Option<String>>,
	/// The Link for the description.
	#[prop(into, optional, default = None.into())]
	description_link: MaybeSignal<Option<String>>,
	/// Contains Action buttons, such as a create, or start/stop button.
	#[prop(into, optional, default = None.into())]
	action_buttons: MaybeSignal<Option<View>>,
) -> impl IntoView {
	view! {
		<div class="w-full flex justify-between items-center">
			<div class="flex flex-col justify-between items-start">
				<TitleContainer>
					<For
						each=move || page_title_items.get()
						key=|item| item.title.clone()
						let:title
					>
						<PageTitle
							icon_position={title.icon_position}
							variant={title.variant}
							to={title.link.unwrap_or_default().clone()}
						>
							{title.title.clone()}
						</PageTitle>
					</For>
				</TitleContainer>

				{
					description_title.get().and_then(|title| {
						Some(
							view! {
								<PageDescription
									description={title}
									doc_link={description_link.get()}
								/>
							}.into_view()
						)
					})
				}
			</div>

			<div class="flex items-center justify-center">
				{
					action_buttons.get().into_view()
				}
			</div>
		</div>
	}
}
