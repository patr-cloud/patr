use std::time::Duration;

use api_models::{
	models::{
		auth::{
			CompleteSignUpRequest,
			CompleteSignUpResponse,
			CreateAccountRequest,
			CreateAccountResponse,
			LoginRequest,
			LoginResponse,
			RecoveryMethod,
			SignUpAccountType,
		},
		workspace::{CreateNewWorkspaceRequest, CreateNewWorkspaceResponse},
	},
	utils::Personal,
	ApiResponse,
};
use reqwest::Client;
use serde::Deserialize;
use tokio::time;

use crate::{app::App, utils::settings::Settings};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SampleDataUser {
	username: String,
	first_name: String,
	last_name: String,
	backup_email: String,
	password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SampleDataWorkspace {
	name: String,
	// domains: Vec<String>,
	super_user: String,
	// users: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SampleData {
	users: Vec<SampleDataUser>,
	workspaces: Vec<SampleDataWorkspace>,
}

pub async fn initialize_sample_data(config: App) {
	// Wait for a second before starting
	log::warn!("Giving server time to initalize before populating data");
	time::sleep(Duration::from_secs(5)).await;
	log::info!("Populating database with sample data...");

	let content = include_str!("../../../assets/sample-data.json");
	let data: SampleData = serde_json::from_str(content).unwrap();

	let client = Client::new();

	// Create all users
	for user in &data.users {
		create_user_account(user, &client, &config.config).await;
	}

	for workspace in &data.workspaces {
		let super_user =
			get_user_by_username(&data.users, &workspace.super_user);
		let response: ApiResponse<LoginResponse> = client
			.post(format!(
				"http://localhost:{}/auth/sign-in",
				config.config.port
			))
			.json(&LoginRequest {
				user_id: super_user.username.clone(),
				password: super_user.password.clone(),
			})
			.send()
			.await
			.unwrap()
			.json()
			.await
			.unwrap();

		let data = match response {
			ApiResponse::Success { success: _, data } => data,
			ApiResponse::Error { success: _, error } => {
				log::error!(
					"Error signing in user `{}`: {:?}",
					super_user.username,
					error
				);
				continue;
			}
		};
		let token = data.access_token;

		let response: ApiResponse<CreateNewWorkspaceResponse> = client
			.post(format!("http://localhost:{}/workspace", config.config.port))
			.header("Authorization", token)
			.json(&CreateNewWorkspaceRequest {
				workspace_name: workspace.name.clone(),
			})
			.send()
			.await
			.unwrap()
			.json()
			.await
			.unwrap();

		match response {
			ApiResponse::Success {
				success: _,
				data: _,
			} => {
				log::info!(
					"workspace `{}` created successfully",
					workspace.name
				);
			}
			ApiResponse::Error { success: _, error } => {
				log::error!(
					"Error creating workspace {}: {:?}",
					workspace.name,
					error
				);
			}
		}
	}
}

async fn create_user_account(
	user: &SampleDataUser,
	client: &Client,
	config: &Settings,
) {
	let response: ApiResponse<CreateAccountResponse> = client
		.post(format!("http://localhost:{}/auth/sign-up", config.port))
		.json(&CreateAccountRequest {
			username: user.username.clone(),
			password: user.password.clone(),
			first_name: user.first_name.clone(),
			last_name: user.last_name.clone(),
			recovery_method: RecoveryMethod::Email {
				recovery_email: user.backup_email.clone(),
			},
			account_type: SignUpAccountType::Personal {
				account_type: Personal,
			},
		})
		.send()
		.await
		.unwrap()
		.json()
		.await
		.unwrap();

	if let ApiResponse::Error { success: _, error } = response {
		log::error!("Error signing up user `{}`: {:?}", user.username, error);
		return;
	}

	let response: ApiResponse<CompleteSignUpResponse> = client
		.post(format!("http://localhost:{}/auth/join", config.port))
		.json(&CompleteSignUpRequest {
			username: user.username.clone(),
			verification_token: "000-000".to_string(),
		})
		.send()
		.await
		.unwrap()
		.json()
		.await
		.unwrap();

	if let ApiResponse::Error { success: _, error } = response {
		log::error!("Error signing up user `{}`: {:?}", user.username, error);
	}
}

fn get_user_by_username<'a>(
	users: &'a [SampleDataUser],
	username: &str,
) -> &'a SampleDataUser {
	users.iter().find(|user| user.username == username).unwrap()
}
