import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { GetDeploymentInfoResponse, ListDeploymentResponse } from "~/bindings";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { deploymentKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useDeploymentsQuery = (page: Accessor<string | undefined>, count: Accessor<string | undefined>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const p = page();
		const c = count();
		return {
			queryKey: deploymentKeys.list(wsId ?? "", p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `?${params.toString()}` : "";

				const response = await httpRequest<ListDeploymentResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					deployments: response.data.deployments,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
				};
			},
		};
	});
};

export const useDeploymentInfoQuery = (id: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<GetDeploymentInfoResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const deploymentId = id();
		return {
			queryKey: deploymentKeys.detail(wsId ?? "", deploymentId),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!deploymentId,
			refetchInterval: (query: { state: { data?: GetDeploymentInfoResponse } }) =>
				query.state.data?.status === "deploying" ? 15_000 : 60_000,
			queryFn: async () => {
				const response = await httpRequest<GetDeploymentInfoResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment/${deploymentId}`,
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
