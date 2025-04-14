/// Alert Component, used to show inline alert in forms and such,
/// e.g., if the user doesn't fill the username while logging in
mod alert;
/// The Button Component, similar to the HTML Button, just with a few extra
/// props to match patr's theme
mod button;
/// All the containers used throughout the application, such as the page
/// container, title container, etc.
mod container;
/// The Icon Component, All the Icons are from feather icons, there's a huge
/// icons sprite in [assets](frontend/assets/icons/sprite/feather-sprite.svg),
/// and the component gets the icon from the sprite
mod icon;
/// Wraps around HTML input, with additional props for styling and such
mod input;
/// Link component to navigate to other pages, wraps around HTML a tag, with
/// additional props for styling and such
mod link;
/// The header for each page. Contains the Title, Description and Tabs.
mod page_header;
/// A Extension of Input, to accommodate features specific to passwords
mod password_input;
/// Status Badge to indicate status of resource
mod status_badge;

pub use self::{
	alert::*,
	button::*,
	container::*,
	icon::*,
	input::*,
	link::*,
	page_header::*,
	password_input::*,
	status_badge::*,
};
