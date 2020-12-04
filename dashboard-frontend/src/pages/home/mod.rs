use yew::{
	prelude::*,
	services::{storage::Area, StorageService},
};
use yew_router::Switch;

#[derive(Switch, Debug, Clone)]
pub enum HomeRouter {
	#[to = "/projects"]
	Projects,
}

#[derive(Properties, Debug, Clone)]
pub struct HomeProps {
	pub route: HomeRouter,
}

pub struct HomeComponent {
	link: ComponentLink<Self>,
	storage: StorageService,
}

impl Component for HomeComponent {
	type Message = i32;
	type Properties = HomeProps;

	fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		let storage = StorageService::new(Area::Local)
			.expect("unable to open localStorage");
		HomeComponent { link, storage }
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		if msg == 1 {}
		false
	}

	fn change(&mut self, _: Self::Properties) -> ShouldRender {
		false
	}

	fn view(&self) -> Html {
		html! {
			<div>
				<h1>
					{"Home page"}
				</h1>
				<input type="button" onclick=self.link.callback(|_| 1) />
			</div>
		}
	}
}
