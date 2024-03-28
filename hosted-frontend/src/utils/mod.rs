#[cfg(not(target_arch = "wasm32"))]
mod client;

#[cfg(not(target_arch = "wasm32"))]
pub use self::client::*;

mod ext;
mod routes;

pub use self::{ext::*, routes::*};
