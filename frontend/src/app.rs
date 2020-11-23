use yew::{
	prelude::*,
	services::{storage::Area, StorageService},
};
use yew_router::{router::Router, Switch};

use crate::{constants::keys, pages::*};

#[derive(Default)]
pub struct UserData {
	pub id: String,
	pub name: String,
	pub username: String,
	pub first_name: String,
	pub last_name: String,
	pub dob: Option<u64>,
	pub bio: Option<String>,
	pub location: Option<String>,
	pub created: u64,
	pub access_token: String,
	pub refresh_token: String,
}

#[allow(dead_code)]
pub struct App {
	link: ComponentLink<Self>,
	storage: StorageService,
	user_data: Option<UserData>,
}

#[derive(Switch, Debug, Clone)]
pub enum MainRouter {
	#[to = "/sign-in"]
	SignIn,
	#[to = "/sign-up"]
	SignUp,
	#[to = "/"]
	Home,
}

#[allow(dead_code)]
pub enum Msg {
	OpenLoginScreen,
	OpenSignUpScreen,
	Login,
	SignUp,
	Logout,
}

impl Component for App {
	type Message = ();
	type Properties = ();

	fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		let storage = StorageService::new(Area::Local)
			.expect("unable to open localStorage");
		let user_data = get_user_data(&storage);
		App {
			link,
			storage,
			user_data,
		}
	}

	fn update(&mut self, _: Self::Message) -> ShouldRender {
		false
	}

	fn change(&mut self, _: Self::Properties) -> ShouldRender {
		false
	}

	fn view(&self) -> Html {
		html! {
			<Router<MainRouter>
				render=Router::render(|switch: MainRouter| {
					match switch {
						MainRouter::SignIn => html! {
							<SignInComponent />
						},
						MainRouter::SignUp => html! {
							<SignUpComponent />
						},
						MainRouter::Home => html! {
							<HomeComponent />
						}
					}
				})
			/>
		}
	}
}

fn get_user_data(storage: &StorageService) -> Option<UserData> {
	let is_logged_in = storage
		.restore::<Result<String, _>>(keys::IS_LOGGED_IN)
		.unwrap_or_else(|_| String::from(keys::FALSE))
		.parse::<bool>()
		.unwrap_or_else(|_| false);

	if is_logged_in {
		let id = if let Ok(id) =
			storage.restore::<Result<String, _>>(keys::USER_ID)
		{
			id
		} else {
			return None;
		};
		let name = if let Ok(name) =
			storage.restore::<Result<String, _>>(keys::NAME)
		{
			name
		} else {
			return None;
		};
		let username = if let Ok(username) =
			storage.restore::<Result<String, _>>(keys::USERNAME)
		{
			username
		} else {
			return None;
		};
		let first_name = if let Ok(first_name) =
			storage.restore::<Result<String, _>>(keys::FIRST_NAME)
		{
			first_name
		} else {
			return None;
		};
		let last_name = if let Ok(last_name) =
			storage.restore::<Result<String, _>>(keys::LAST_NAME)
		{
			last_name
		} else {
			return None;
		};
		let dob =
			if let Ok(dob) = storage.restore::<Result<String, _>>(keys::DOB) {
				Some(dob.parse::<u64>().unwrap())
			} else {
				None
			};
		let bio =
			if let Ok(bio) = storage.restore::<Result<String, _>>(keys::BIO) {
				Some(bio)
			} else {
				None
			};
		let location = if let Ok(location) =
			storage.restore::<Result<String, _>>(keys::LOCATION)
		{
			Some(location)
		} else {
			None
		};
		let created = if let Ok(created) =
			storage.restore::<Result<String, _>>(keys::CREATED)
		{
			created
		} else {
			return None;
		}
		.parse::<u64>()
		.unwrap();
		let access_token = if let Ok(access_token) =
			storage.restore::<Result<String, _>>(keys::ACCESS_TOKEN)
		{
			access_token
		} else {
			return None;
		};
		let refresh_token = if let Ok(refresh_token) =
			storage.restore::<Result<String, _>>(keys::REFRESH_TOKEN)
		{
			refresh_token
		} else {
			return None;
		};

		Some(UserData {
			id,
			name,
			username,
			first_name,
			last_name,
			dob,
			bio,
			location,
			created,
			access_token,
			refresh_token,
		})
	} else {
		None
	}
}
