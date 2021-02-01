use yew::{html, Component, ComponentLink, Html, Properties, ShouldRender};

#[allow(dead_code)]
pub struct Sidebar {
	link: ComponentLink<Self>,
	props: SidebarProps,
}

#[derive(Properties, Clone)]
pub struct SidebarProps {
	pub selected_item: String,
}

impl Component for Sidebar {
	type Message = ();
	type Properties = SidebarProps;

	fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
		Sidebar { link, props }
	}

	fn update(&mut self, _: Self::Message) -> ShouldRender {
		false
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		self.props = props;
		true
	}

	fn view(&self) -> Html {
		html! {
			<div class="row collection">
				{{
					[
						"My Drive",
						"Shared with me",
						"Recent",
						"Starred",
						"Trash",
					].iter().map(|item| {
						if &self.props.selected_item == item {
							html! {
								<a href="#!" class="collection-item white-text active">{{item}}</a>
							}
						} else {
							html! {
								<a href="#!" class="collection-item black-text">{{item}}</a>
							}
						}
					}).collect::<Vec<_>>()
				}}
			</div>
		}
	}
}
