pub mod config;
pub mod extractors;
pub mod layers;

mod router_ext;

pub use self::router_ext::RouterExt;

pub mod constants {
    pub const JWT_ISSUER: &str = "https://api.patr.cloud";
}
