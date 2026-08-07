//! A CLI tool for interacting and managing your Patr resources.
//!
//! This is a thin wrapper around the `cli` library — see `lib.rs`.

#[tokio::main]
async fn main() -> std::process::ExitCode {
	use std::process::ExitCode;

	use clap::Parser;
	use cli::{commands, prelude::*};
	use models::{ApiErrorResponseBody, utils::False};

	let AppArgs { args, command } = AppArgs::parse();

	let state = AppState::load()
		.inspect_err(|err| {
			eprintln!("{err}");
			eprintln!("Loading default state...");
		})
		.unwrap_or_default();

	let output_format = args.output;
	let Ok(output) = commands::execute(command, args, state)
		.await
		.map_err(|err| {
			let error_response = ApiErrorResponseBody {
				success: False,
				error: match &err {
					AppError::ApiError(err) => *err,
					other => ErrorType::server_error(other),
				},
				message: err.to_string(),
			};
			eprintln!(
				"{}",
				match output_format {
					OutputType::Text => {
						err.to_string()
					}
					OutputType::Json => {
						serde_json::to_string(&error_response).expect("Failed to serialize error")
					}
					OutputType::PrettyJson => {
						serde_json::to_string_pretty(&error_response)
							.expect("Failed to serialize error")
					}
				}
			)
		})
	else {
		return ExitCode::FAILURE;
	};

	match output_format {
		OutputType::Text => {
			eprintln!("{}", output.text)
		}
		OutputType::Json => {
			println!(
				"{}",
				serde_json::to_string(&output.json).expect("Failed to serialize")
			);
		}
		OutputType::PrettyJson => {
			println!(
				"{}",
				serde_json::to_string_pretty(&output.json).expect("Failed to serialize")
			);
		}
	}

	ExitCode::SUCCESS
}
