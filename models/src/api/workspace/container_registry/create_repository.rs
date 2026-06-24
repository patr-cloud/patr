use crate::{prelude::*, utils::constants::CONTAINER_REGISTRY_REPOSITORY_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Creates a new container repository in the workspace.
	CreateContainerRepository,
	POST "/workspace/{workspace_id}/container-registry" {
		/// The workspace to create the container repository in.
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Create),
		}
	},
	request = {
		/// The name of the repository to create.
		#[preprocess(trim, length(min = 4, max = 255), regex = CONTAINER_REGISTRY_REPOSITORY_NAME_REGEX)]
		pub name: String,
	},
	response = {
		/// The id of the created repository.
		#[serde(flatten)]
		pub id: OnlyId,
	},
	audit_log = AppAuditLogger {
		audit_log_type: AuditLogType::ResourceCreated,
		resource_type: ResourceType::ContainerRegistryRepository,
		extract_resource_id: ResourceIdExtractor::FromResponse(|res| res.body.id.id),
	},
);
