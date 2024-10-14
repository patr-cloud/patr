/// The Alert component.
///
/// The alert component is used to display an alert to the user. It is used to
/// show the user a message, like a success message, a warning message, or an
/// error message. The alert disappeares after a few seconds
pub mod alert;
/// The backdrop component.
///
/// The backdrop component is used to display a backdrop that can be used to
/// block the user from interacting with the rest of the page. It is used to
/// show the user that something is loading or in modals
pub mod backdrop;
/// The Checkbox Dropdown component.
///
/// The checkbox dropdown component is used to display a dropdown with
/// checkboxes.
pub mod checkbox_dropdown;
/// The containers module.
///
/// This module contains all the container components. A container is a
/// component that can be used to hold other components (like a box).
pub mod containers;
/// The dashboard container module.
///
/// This module contains the dashboard container component. The dashboard
/// container is a container that is used to hold dashboard components.
pub mod dashboard_container;
/// The Double Input Slider component.
///
/// The Double Input Slider component is used to display a slider with two
/// inputs. It is used to allow the user to select two values from a range of
/// values.
pub mod double_input_slider;
/// The error page component.
///
/// The error page component is used to display an error page. It is used to
/// show the user an error page, like a 404 error page, or any other form of
/// error.
pub mod error_page;
/// The Icon component.
///
/// This component is used to display an icon in the fontawsome library, of
/// different sizes and colors.
pub mod icon;
/// The input component.
///
/// The input component is used for text, email, and other things.
pub mod input;
/// The Input Dropdown component.
///
/// The input dropdown component is used to display a dropdown input. It is used
/// to allow the user to select an option from a list of options.
pub mod input_dropdown;
/// The link component.
///
/// The link component is used to create a link to another page, or to an
/// external website. It can also be used to create a button.
pub mod link;
/// The Modal component.
///
/// The modal component is used to display a modal. It is used to show the user
/// a modal, like a confirmation modal, or a settings modal.
pub mod modal;
/// The number picker component.
///
/// The number picker component is used to display a number picker. It is used
/// to allow the user to select a number from a range of numbers.
pub mod number_picker;
/// The OTP input component.
///
/// The OTP input component is used to display an input for an OTP code. It is
/// used in the login page.
pub mod otp_input;
/// The page title component.
///
/// The page title component is used to display the title of a page,
/// specifically the title of a dashboard page.
pub mod page_title;
/// The Popover Component
///
/// The Popover Component, used to display a tooltip when user hovers / clicks
/// over an element. It is used to show the user more information about an
/// element.
pub mod popover;
/// The sidebar component.
///
/// The sidebar component is used to display a sidebar that can be used to
/// navigate between different pages. It is used in the dashboard, and will
/// reactively change the page when a link is clicked or when a new page is
/// loaded.
pub mod sidebar;
/// The skeleton component.
///
/// The skeleton component is used to display a loading skeleton for a component
/// that is loading. It is used to show the user that something is loading, and
/// that they should wait. This is not needed for server side rendered
/// components, but doesn't hurt to have since it will be replaced by the actual
/// component when it loads.
pub mod skeleton;
/// The spinner component.
///
/// The spinner component is used to display a loading spinner. It is used to
/// show the user that something is loading, and that they should wait. This is
/// not needed for situations when javascript / WASM hasn't loaded yet, but can
/// be kept since those situations do a full page reload anyway.
pub mod spinner;
/// The status badge component.
///
/// The status badge component is used to display a status badge, like a success
/// badge, a warning badge, or an error badge. It is used to show the user the
/// status of something, like a database, or a deployment.
pub mod status_badge;
/// The table dashboard component.
///
/// The table dashboard component is used to display a table of data in a
/// dashboard. It is used to show the user a table of data, like a list of
/// users, or a list of deployments, etc.
pub mod table_dashboard;
/// The Textbox component.
///
/// The textbox component is used to display a textbox input when the user
/// cannot edit the value.
pub mod textbox;
/// The toast component.
///
/// The toast component is used to display a notification to the user. For
/// Example, to show the user a notification for success or failure of an action
/// like reseouce creation or deletion.
pub mod toast;
/// The Tooltip component.
///
/// The tooltip component is used to display a tooltip when the user hovers over
/// an element. It is used to show the user more information about an element.
pub mod tooltip;
