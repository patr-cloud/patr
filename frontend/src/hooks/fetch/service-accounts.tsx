import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { GetServiceAccountInfoResponse, ListServiceAccountsResponse } from "~/bindings";

import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { serviceAccountKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useServiceAccountsQuery = (page: Accessor<string | undefined>, count: Accessor<string | undefined>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const p = page();
		const c = count();
		return {
			queryKey: serviceAccountKeys.list(wsId ?? "", p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch service accounts" },
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `?${params.toString()}` : "";

				const response = await httpRequest<ListServiceAccountsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/service-account${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					serviceAccounts: response.data.serviceAccounts || [],
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
				};
			},
		};
	});
};

export const useServiceAccountQuery = (id: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<GetServiceAccountInfoResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const serviceAccountId = id();
		return {
			queryKey: serviceAccountKeys.detail(wsId ?? "", serviceAccountId),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!serviceAccountId,
			meta: { errorMessage: "Failed to fetch service account info" },
			queryFn: async () => {
				const response = await httpRequest<GetServiceAccountInfoResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/service-account/${serviceAccountId}`,
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
