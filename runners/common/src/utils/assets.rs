use rust_embed::Embed;

/// This struct embeds the binaries used for cloudflared and nginx. This is used
/// to use the binaries in the `runners` without having to ship them with the
/// binary.
#[derive(Embed)]
#[folder = "../../assets/binaries"]
pub struct Binaries;
