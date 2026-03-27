import usePermissionsQuery from "./permissions";
import useWorkspacesQuery from "./workspaces";
import useUserPermissionsQuery from "./user-permissions";
import { useDeploymentsQuery, useDeploymentInfoQuery } from "./deployments";
import { useRunnersQuery, useRunnersListQuery } from "./runners";
import { useWorkspaceInfoQuery } from "./workspace";
import { useRolesQuery, useAllRolesQuery } from "./roles";
import { useMembersQuery } from "./members";
import { useApiTokensQuery } from "./api-tokens";
import { useContainerRegistriesQuery } from "./container-registry";
import { useDomainsQuery, useDomainVerificationRecordsQuery } from "./domains";

export {
	useDeploymentsQuery,
	useDeploymentInfoQuery,
	useRunnersQuery,
	useRunnersListQuery,
	usePermissionsQuery,
	useUserPermissionsQuery,
	useWorkspacesQuery,
	useWorkspaceInfoQuery,
	useRolesQuery,
	useAllRolesQuery,
	useMembersQuery,
	useApiTokensQuery,
	useContainerRegistriesQuery,
	useDomainsQuery,
	useDomainVerificationRecordsQuery,
};
