/// The endpoint to activate MFA for a user
mod activate_mfa;
/// The endpoint to deactivate MFA for a user
mod deactivate_mfa;
/// The endpoint to get the MFA secret for a user while activating MFA
mod get_mfa_secret;

pub use self::{activate_mfa::*, deactivate_mfa::*, get_mfa_secret::*};
