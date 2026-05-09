//! Regenerate the SeaORM entity files under `api/src/entities/`.
//!
//! Usage: `cargo entities`
//!
//! Reads `DATABASE_URL` from the environment (same convention as `cargo
//! prepare`). The DB it points at must already have the patr schema applied
//! — run `cargo run --bin api -- --migrate` against it first.

use std::{env, process};

fn main() {
	if env::var_os("DATABASE_URL").is_none() {
		eprintln!("Error: DATABASE_URL is not set.");
		eprintln!();
		eprintln!("sea-orm-cli reads DATABASE_URL the same way sqlx-cli does. Set it to a");
		eprintln!("Postgres URL where the patr schema is already applied, e.g.:");
		eprintln!();
		eprintln!("  DATABASE_URL=postgres://user:password@localhost:15432/api cargo entities");
		process::exit(1);
	}

	// CARGO_MANIFEST_DIR is set by cargo at run time and is already an absolute
	// path to this crate's directory. We read it at runtime (not via env!()) so
	// the cached binary stays correct if the worktree moves.
	let manifest_dir = env::var("CARGO_MANIFEST_DIR")
		.expect("CARGO_MANIFEST_DIR not set; run via `cargo entities`");
	let entities_dir = format!("{manifest_dir}/src/entities");

	let status = match process::Command::new("sea-orm-cli")
		.args([
			"generate",
			"entity",
			"-o",
			&entities_dir,
			"--entity-format",
			"compact",
			"--date-time-crate",
			"time",
			"--with-prelude",
			"all",
			"--ignore-tables",
			"migrations,spatial_ref_sys",
		])
		.status()
	{
		Ok(status) => status,
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
			eprintln!("Error: sea-orm-cli not found in PATH.");
			eprintln!();
			eprintln!("Install it with:");
			eprintln!("  cargo install sea-orm-cli@2.0.0-rc.38 --locked");
			process::exit(1);
		}
		Err(err) => {
			eprintln!("Error launching sea-orm-cli: {err}");
			process::exit(1);
		}
	};

	process::exit(status.code().unwrap_or(1));
}
