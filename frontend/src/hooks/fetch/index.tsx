import useApiEnvironmentQuery from "./api-environment";
import usePermissionsQuery from "./permissions";
import useResourcesInfoQuery from "./resources";
import useWorkspacesQuery from "./workspaces";
import useUserPermissionsQuery from "./user-permissions";
import { useDeploymentsQuery, useDeploymentInfoQuery } from "./deployments";
import {
	useRunnersQuery,
	useRunnersInfiniteQuery,
	useRunnerInfoQuery,
	useRunnersListQuery,
	useRunnerDeploymentsQuery,
} from "./runners";
import { useWorkspaceInfoQuery } from "./workspace";
import { useRolesQuery, useAllRolesQuery, useRoleInfoQuery, useRoleBindingsQuery } from "./roles";
import { useMembersQuery } from "./members";
import { useInvitesQuery } from "./invitations";
import { useApiTokensQuery, useApiTokenInfoQuery } from "./api-tokens";
import { useOAuthGrantsQuery } from "./oauth-grants";
import {
	useContainerRegistriesQuery,
	useContainerRegistriesInfiniteQuery,
	useContainerRegistryInfoQuery,
	useContainerManifestsQuery,
	useContainerManifestDetailsQuery,
	useContainerExposedPortsQuery,
	useContainerRegistryUsageQuery,
	useContainerTagsQuery,
} from "./container-registry";
import { useDomainsQuery, useDomainInfoQuery } from "./domains";
import { useSecretsQuery, useSecretInfoQuery } from "./secrets";
import { useServiceAccountsQuery, useServiceAccountQuery } from "./service-accounts";
import { useManagedUrlsQuery } from "./managed-urls";
import { useRunnerMetricsQuery, useDeploymentMetricsQuery } from "./metrics";
import { useUserInfoQuery, useMfaSecretQuery } from "./user";
import { useApiVersionQuery } from "./version";

export {
	useApiEnvironmentQuery,
	useDeploymentsQuery,
	useDeploymentInfoQuery,
	useDeploymentMetricsQuery,
	useRunnersQuery,
	useRunnersInfiniteQuery,
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
	useRoleBindingsQuery,
	useMembersQuery,
	useInvitesQuery,
	useApiTokensQuery,
	useApiTokenInfoQuery,
	useOAuthGrantsQuery,
	useContainerRegistriesQuery,
	useContainerRegistriesInfiniteQuery,
	useContainerRegistryInfoQuery,
	useContainerManifestsQuery,
	useContainerManifestDetailsQuery,
	useContainerExposedPortsQuery,
	useContainerRegistryUsageQuery,
	useContainerTagsQuery,
	useDomainsQuery,
	useDomainInfoQuery,
	useSecretsQuery,
	useSecretInfoQuery,
	useServiceAccountsQuery,
	useServiceAccountQuery,
	useManagedUrlsQuery,
	useUserInfoQuery,
	useMfaSecretQuery,
	useApiVersionQuery,
};
