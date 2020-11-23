use yew::{
	prelude::*,
	services::{storage::Area, StorageService},
};

pub struct SignInComponent {
	link: ComponentLink<Self>,
	storage: StorageService,
}

impl Component for SignInComponent {
	type Message = ();
	type Properties = ();

	fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		let storage = StorageService::new(Area::Local)
			.expect("unable to open localStorage");
		SignInComponent { link, storage }
	}

	fn update(&mut self, _: Self::Message) -> ShouldRender {
		false
	}

	fn change(&mut self, _props: Self::Properties) -> ShouldRender {
		false
	}

	fn view(&self) -> Html {
		html! {
			<h1>
				{"Sign In page"}
			</h1>
		}
	}
}
