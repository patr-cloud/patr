/// Contains the Description of the page
mod page_description;
/// The Title of the Page
mod page_title;
/// Various Tabs to navigate to different pages in the same group
mod tabs;
/// Encapsulates the Title and Description of the Page
mod title_container;

pub use self::{page_description::*, page_title::*, tabs::*, title_container::*};
