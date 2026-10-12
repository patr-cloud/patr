import { createInfiniteQuery, createQuery, keepPreviousData } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import {
	Deployment,
	GetRunnerInfoResponse,
	ListDeploymentResponse,
	ListRunnersForWorkspaceResponse,
	Runner,
	WithId,
} from "~/bindings";

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
			meta: { errorMessage: "Failed to fetch runners" },
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

/** How many runners a picker loads per page as it's scrolled. */
const RUNNER_PICKER_PAGE_SIZE = 20;

/** One page of runners, with the total so the next page can be worked out. */
type RunnerPage = {
	runners: WithId<Runner>[];
	totalCount: number;
	page: number;
};

/**
 * Loads a workspace's runners a page at a time, for pickers that load the
 * next page as they're scrolled. An optional [search] narrows them by name on
 * the server. List pages use [useRunnersListQuery].
 */
export const useRunnersInfiniteQuery = (search?: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createInfiniteQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const s = search?.() ?? "";
		const searchParam = s ? `&search[name]=${encodeURIComponent(s)}` : "";
		return {
			queryKey: runnerKeys.infiniteList(wsId ?? "", s),
			// A new search re-keys this. Keep the previous results up until the
			// new ones land, rather than blanking the open picker.
			placeholderData: keepPreviousData,
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch runners" },
			initialPageParam: 0,
			queryFn: async ({ pageParam }: { pageParam: number }): Promise<RunnerPage> => {
				const response = await httpRequest<ListRunnersForWorkspaceResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner?page=${pageParam}&count=${RUNNER_PICKER_PAGE_SIZE}${searchParam}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					runners: response.data.runners,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
					page: pageParam,
				};
			},
			getNextPageParam: (lastPage: RunnerPage): number | undefined => {
				const loaded = (lastPage.page + 1) * RUNNER_PICKER_PAGE_SIZE;
				return loaded < lastPage.totalCount ? lastPage.page + 1 : undefined;
			},
		};
	});
};

export const useRunnerInfoQuery = (id: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<GetRunnerInfoResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const runnerId = id();
		return {
			queryKey: runnerKeys.detail(wsId ?? "", runnerId),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!runnerId,
			meta: { errorMessage: "Failed to fetch runner info" },
			queryFn: async () => {
				const response = await httpRequest<GetRunnerInfoResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner/${runnerId}`,
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
			meta: { errorMessage: "Failed to fetch runners" },
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

export const useRunnerDeploymentsQuery = (
	runnerId: Accessor<string>,
	page: Accessor<number>,
	count: Accessor<number>
) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<{ deployments: WithId<Deployment>[]; totalCount: number }>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const rid = runnerId();
		const p = page();
		const c = count();
		return {
			queryKey: runnerKeys.deployments(wsId ?? "", rid, p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!rid,
			meta: { errorMessage: "Failed to fetch deployments for runner" },
			queryFn: async () => {
				const response = await httpRequest<ListDeploymentResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment?search[runner]=${rid}&page=${p}&count=${c}`,
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
