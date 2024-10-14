/// All auth related endpoints, including OAuth
mod auth;
#[cfg(not(target_arch = "wasm32"))]
/// Contains the middlewares that will be used with server_fns
mod middlewares;
/// All endpoints that relate to a user and their data
mod user;
/// All endpoints that can be performed on a workspace
mod workspace;

#[cfg(not(target_arch = "wasm32"))]
pub use self::middlewares::*;
pub use self::{auth::*, user::*, workspace::*};
