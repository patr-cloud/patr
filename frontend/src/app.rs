use yew::{
	prelude::*,
	services::{storage::Area, StorageService},
};

use crate::constants::keys;

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

pub struct App {
	link: ComponentLink<Self>,
	storage: StorageService,
	user_data: Option<UserData>,
}

pub enum Msg {
	OpenLoginScreen,
	OpenSignUpScreen,
	Login,
	SignUp,
	Logout,
}

impl Component for App {
	type Message = Msg;
	type Properties = ();

	fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		let storage = StorageService::new(Area::Local);
		let user_data = get_user_data(&storage);
		App {
			link,
			storage,
			user_data,
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		if let Msg::Login = msg {
			self.user_data = Some(Default::default());
			return true;
		} else if let Msg::Logout = msg {
			self.user_data = None;
			return true;
		} else {
			return false;
		}
	}

	fn view(&self) -> Html {
		if self.user_data.is_none() {
			html! {
				<>
					<h1>{"Login"}</h1>
					<div class="container row">
						<input type="email" name="userId" class="col s3" />
						<input type="password" name="password" class="col s3" />
						<input type="button" value="Login" class="button col s3" onclick=self.link.callback(|_| Msg::Login) />
					</div>
				</>
			}
		} else {
			html! {
				<>
					<h1>{"Logged in"}</h1>
					<div class="container row">
						<input type="email" name="userId" class="col s3" />
						<input type="password" name="password" class="col s3" />
						<input type="button" value="Logout" class="button col s3" onclick=self.link.callback(|_| Msg::Logout) />
					</div>
				</>
			}
		}
	}
}

fn get_user_data(storage: &StorageService) -> Option<UserData> {
	let is_logged_in = storage
		.restore::<Result<String, _>>(keys::IS_LOGGED_IN)
		.unwrap_or_else(|_| String::from(keys::FALSE))
		.parse::<bool>()
		.unwrap();

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
