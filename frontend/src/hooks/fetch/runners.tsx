import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { ListRunnersForWorkspaceResponse } from "~/bindings";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { runnerKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useRunnersQuery = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<ListRunnersForWorkspaceResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		return {
			queryKey: runnerKeys.list(wsId ?? ""),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			queryFn: async () => {
				const response = await httpRequest<ListRunnersForWorkspaceResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner`,
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

export const useRunnersListQuery = (page: Accessor<string | undefined>, count: Accessor<string | undefined>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const p = page();
		const c = count();
		return {
			queryKey: runnerKeys.pagedList(wsId ?? "", p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `?${params.toString()}` : "";

				const response = await httpRequest<ListRunnersForWorkspaceResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					runners: response.data.runners,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
				};
			},
		};
	});
};
