import useApiEnvironmentQuery from "./api-environment";
import usePermissionsQuery from "./permissions";
import useWorkspacesQuery from "./workspaces";
import useUserPermissionsQuery from "./user-permissions";
import { useDeploymentsQuery, useDeploymentInfoQuery } from "./deployments";
import { useRunnersQuery, useRunnerInfoQuery, useRunnersListQuery, useRunnerDeploymentsQuery } from "./runners";
import { useWorkspaceInfoQuery } from "./workspace";
import { useRolesQuery, useAllRolesQuery, useRoleInfoQuery, useRoleUsersQuery } from "./roles";
import { useMembersQuery, useWorkspaceOwnerQuery } from "./members";
import { useApiTokensQuery, useApiTokenInfoQuery } from "./api-tokens";
import {
	useContainerRegistriesQuery,
	useContainerRegistryInfoQuery,
	useContainerManifestsQuery,
	useContainerManifestDetailsQuery,
	useContainerExposedPortsQuery,
	useContainerRegistryUsageQuery,
	useContainerTagsQuery,
} from "./container-registry";
import { useDomainsQuery, useDomainInfoQuery } from "./domains";
import { useManagedUrlsQuery } from "./managed-urls";
import { useRunnerMetricsQuery, useDeploymentMetricsQuery } from "./metrics";
import { useUserInfoQuery, useUserSearchQuery, useMfaSecretQuery } from "./user";
import { useApiVersionQuery } from "./version";

export {
	useApiEnvironmentQuery,
	useDeploymentsQuery,
	useDeploymentInfoQuery,
	useDeploymentMetricsQuery,
	useRunnersQuery,
	useRunnerInfoQuery,
	useRunnersListQuery,
	useRunnerDeploymentsQuery,
	useRunnerMetricsQuery,
	usePermissionsQuery,
	useUserPermissionsQuery,
	useWorkspacesQuery,
	useWorkspaceInfoQuery,
	useRolesQuery,
	useAllRolesQuery,
	useRoleInfoQuery,
	useRoleUsersQuery,
	useMembersQuery,
	useWorkspaceOwnerQuery,
	useApiTokensQuery,
	useApiTokenInfoQuery,
	useContainerRegistriesQuery,
	useContainerRegistryInfoQuery,
	useContainerManifestsQuery,
	useContainerManifestDetailsQuery,
	useContainerExposedPortsQuery,
	useContainerRegistryUsageQuery,
	useContainerTagsQuery,
	useDomainsQuery,
	useDomainInfoQuery,
	useManagedUrlsQuery,
	useUserInfoQuery,
	useUserSearchQuery,
	useMfaSecretQuery,
	useApiVersionQuery,
};
