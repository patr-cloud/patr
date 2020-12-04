use yew::prelude::*;

pub struct App;

impl Component for App {
	type Message = ();
	type Properties = ();

	fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
		App
	}

	fn update(&mut self, _: Self::Message) -> ShouldRender {
		false
	}

	fn change(&mut self, _: Self::Properties) -> ShouldRender {
		false
	}

	fn view(&self) -> Html {
		html! {
			// <Router<MainRouter>
			// 	render=Router::render(|switch: MainRouter| {
			// 		match switch {
			// 			MainRouter::SignIn => html! {
			// 				<SignInComponent />
			// 			},
			// 			MainRouter::SignUp => html! {
			// 				<SignUpComponent />
			// 			},
			// 			MainRouter::Home(home_route) => html! {
			// 				<HomeComponent route = home_route/>
			// 			}
			// 		}
			// 	})
			// />
			<div>
				{"HTML CONTENT GOES HERE"}
			</div>
		}
	}
}

fn main() {
	yew::start_app::<App>();
}
