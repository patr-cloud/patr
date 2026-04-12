import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { GetRunnerInfoResponse, ListRunnersForWorkspaceResponse } from "~/bindings";
import { useToast } from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { runnerKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useRunnersQuery = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

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
					toast("Failed to fetch runners", "error");
					throw new Error(response.data.error);
				}

				return response.data;
			},
		};
	});
};

export const useRunnerInfoQuery = (id: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	return createQuery<GetRunnerInfoResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const runnerId = id();
		return {
			queryKey: runnerKeys.detail(wsId ?? "", runnerId),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!runnerId,
			queryFn: async () => {
				const response = await httpRequest<GetRunnerInfoResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner/${runnerId}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					toast("Failed to fetch runner info", "error");
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
	const toast = useToast();

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
					toast("Failed to fetch runners", "error");
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
