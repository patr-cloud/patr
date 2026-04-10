//! Migration file generator.
//!
//! Usage: `cargo migrate <name>`
//!
//! Creates a new migration file for the current workspace version with
//! boilerplate code and registers it in the version's `mod.rs`.

use std::{env, fs, io::Write, path::PathBuf, process};

fn main() {
	let Some(name) = env::args().nth(1) else {
		eprintln!("Usage: cargo migration <name>");
		eprintln!("  e.g. cargo migration add_user_avatars");
		process::exit(1);
	};

	// Validate name: lowercase alphanumeric + underscores only
	if !name
		.chars()
		.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
	{
		eprintln!("Error: migration name must be lowercase alphanumeric with underscores");
		process::exit(1);
	}

	let major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u64>().unwrap();
	let minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u64>().unwrap();
	let patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u64>().unwrap();

	let version_dir_name = format!("v{major}_{minor}_{patch}");
	let migrations_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/migrations");
	let version_dir = migrations_dir.join(&version_dir_name);

	// Create version directory if it doesn't exist
	let is_new_version = !version_dir.exists();
	if is_new_version {
		fs::create_dir_all(&version_dir).unwrap_or_else(|e| {
			eprintln!("Error creating directory {}: {e}", version_dir.display());
			process::exit(1);
		});
	}

	// Find the next migration number
	let next_num = {
		let mut max_num = 0u32;
		if let Ok(entries) = fs::read_dir(&version_dir) {
			for entry in entries.flatten() {
				let name = entry.file_name();
				let name = name.to_string_lossy();
				if name.starts_with('m') && name.ends_with(".rs") && name != "mod.rs" {
					// Parse "m001_..." -> 1
					if let Some(num_str) = name.strip_prefix('m') {
						if let Some(num_str) = num_str.split('_').next() {
							if let Ok(num) = num_str.parse::<u32>() {
								max_num = max_num.max(num);
							}
						}
					}
				}
			}
		}
		max_num + 1
	};
	let migration_name = format!("m{next_num:03}_{name}");
	let file_name = format!("{migration_name}.rs");
	let file_path = version_dir.join(&file_name);

	// Write the migration file
	let content = generate_migration();
	fs::write(&file_path, content).unwrap_or_else(|e| {
		eprintln!("Error writing {}: {e}", file_path.display());
		process::exit(1);
	});

	// Update the version's mod.rs
	let version_mod_path = version_dir.join("mod.rs");
	let mod_line = format!("mod {migration_name};\n");
	if version_mod_path.exists() {
		fs::OpenOptions::new()
			.append(true)
			.open(&version_mod_path)
			.unwrap()
			.write(mod_line.as_bytes())
			.unwrap();
	} else {
		let header = format!("/// Migrations for version {major}.{minor}.{patch}\n");
		fs::write(&version_mod_path, format!("{header}{mod_line}")).unwrap();
	}

	// If this is a new version, register it in the top-level mod.rs
	if is_new_version {
		let top_mod_path = migrations_dir.join("mod.rs");
		let top_mod = fs::read_to_string(&top_mod_path).unwrap();
		let new_mod_line = format!("mod {version_dir_name};\n");
		if !top_mod.contains(&new_mod_line) {
			// Insert after the last `mod v*;` line
			let insert_pos = top_mod
				.lines()
				.enumerate()
				.filter(|(_, line)| line.starts_with("mod v"))
				.last()
				.map(|(i, _)| {
					top_mod
						.lines()
						.take(i + 1)
						.map(|l| l.len() + 1)
						.sum::<usize>()
				})
				.unwrap_or_else(|| {
					// No existing version mods — insert after the
					// top-level mod.rs imports
					top_mod.find("\n\n").map(|p| p + 2).unwrap_or(top_mod.len())
				});

			let mut new_top_mod = top_mod.clone();
			new_top_mod.insert_str(insert_pos, &new_mod_line);
			fs::write(&top_mod_path, new_top_mod).unwrap();
		}
	}

	println!("Created: {}", file_path.display());
	println!("Migration: {version_dir_name}/{migration_name} (version {major}.{minor}.{patch})");
}

fn generate_migration() -> String {
	r#"//! TODO: describe this migration

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	todo!("Implement migration")
}
"#
	.to_string()
}
