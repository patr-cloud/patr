import { createInfiniteQuery, createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { GetSecretInfoResponse, ListSecretDeploymentsResponse, Secret, SecretDeployment, WithId } from "~/bindings";

import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { secretKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

type ListSecretsForWorkspaceResponse = {
	secrets: WithId<Secret>[];
};

export const useSecretsQuery = (page: Accessor<string | undefined>, count: Accessor<string | undefined>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const p = page();
		const c = count();
		return {
			queryKey: secretKeys.list(wsId ?? "", p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch secrets" },
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `?${params.toString()}` : "";

				const response = await httpRequest<ListSecretsForWorkspaceResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/secret${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					secrets: response.data.secrets || [],
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
				};
			},
		};
	});
};

export const useSecretInfoQuery = (id: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<GetSecretInfoResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const secretId = id();
		return {
			queryKey: secretKeys.detail(wsId ?? "", secretId),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!secretId,
			meta: { errorMessage: "Failed to fetch secret info" },
			queryFn: async () => {
				const response = await httpRequest<GetSecretInfoResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/secret/${secretId}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return response.data;
			},
		};
	});
};

const SECRET_DEPLOYMENTS_PAGE_SIZE = 25;

type SecretDeploymentPage = {
	deployments: WithId<SecretDeployment>[];
	totalCount: number;
	page: number;
};

/** Loads the deployments using a secret a page at a time, as the list is scrolled. */
export const useSecretDeploymentsQuery = (id: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createInfiniteQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const secretId = id();
		return {
			queryKey: secretKeys.deployments(wsId ?? "", secretId),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!secretId,
			meta: { errorMessage: "Failed to fetch the deployments using this secret" },
			initialPageParam: 0,
			queryFn: async ({ pageParam }: { pageParam: number }): Promise<SecretDeploymentPage> => {
				const response = await httpRequest<ListSecretDeploymentsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/secret/${secretId}/deployment?page=${pageParam}&count=${SECRET_DEPLOYMENTS_PAGE_SIZE}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					deployments: response.data.deployments,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
					page: pageParam,
				};
			},
			getNextPageParam: (lastPage: SecretDeploymentPage): number | undefined => {
				const loaded = (lastPage.page + 1) * SECRET_DEPLOYMENTS_PAGE_SIZE;
				return loaded < lastPage.totalCount ? lastPage.page + 1 : undefined;
			},
		};
	});
};
