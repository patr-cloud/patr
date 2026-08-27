import useApiEnvironmentQuery from "./api-environment";
import usePermissionsQuery from "./permissions";
import useResourcesInfoQuery from "./resources";
import useWorkspacesQuery from "./workspaces";
import useUserPermissionsQuery from "./user-permissions";
import { useDeploymentsQuery, useDeploymentInfoQuery } from "./deployments";
import { useRunnersQuery, useRunnerInfoQuery, useRunnersListQuery, useRunnerDeploymentsQuery } from "./runners";
import { useWorkspaceInfoQuery } from "./workspace";
import { useRolesQuery, useAllRolesQuery, useRoleInfoQuery, useRoleUsersQuery } from "./roles";
import { useMembersQuery } from "./members";
import { useInvitesQuery } from "./invitations";
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
import { useUserInfoQuery, useMfaSecretQuery } from "./user";

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
	useResourcesInfoQuery,
	useUserPermissionsQuery,
	useWorkspacesQuery,
	useWorkspaceInfoQuery,
	useRolesQuery,
	useAllRolesQuery,
	useRoleInfoQuery,
	useRoleUsersQuery,
	useMembersQuery,
	useInvitesQuery,
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
	useMfaSecretQuery,
};
