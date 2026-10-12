import { createInfiniteQuery, createQuery, keepPreviousData } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import {
	ContainerRepository,
	GetContainerRegistryUsageResponse,
	GetContainerRepositoryExposedPortsResponse,
	GetContainerRepositoryInfoResponse,
	GetContainerRepositoryManifestDetailsResponse,
	ListContainerRepositoriesResponse,
	ListContainerRepositoryManifestsResponse,
	ListContainerRepositoryTagsResponse,
	WithId,
} from "~/bindings";

import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { containerRegistryKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useContainerRegistriesQuery = (
	page: Accessor<string | undefined>,
	count: Accessor<string | undefined>
) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const p = page();
		const c = count();
		return {
			queryKey: containerRegistryKeys.list(wsId ?? "", p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch container registries" },
			queryFn: async () => {
				const params = new URLSearchParams();
				if (p) params.set("page", p);
				if (c) params.set("count", c);
				const qs = params.size > 0 ? `?${params.toString()}` : "";

				const response = await httpRequest<ListContainerRepositoriesResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry${qs}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					repositories: response.data.repositories,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
				};
			},
		};
	});
};

/** How many repositories a picker loads per page as it's scrolled. */
const REPOSITORY_PICKER_PAGE_SIZE = 20;

/** One page of repositories, with the total so the next page can be worked out. */
type RepositoryPage = {
	repositories: WithId<ContainerRepository>[];
	totalCount: number;
	page: number;
};

/**
 * Loads a workspace's repositories a page at a time, for pickers that load
 * the next page as they're scrolled. An optional [search] narrows them by name
 * on the server. List pages use [useContainerRegistriesQuery].
 */
export const useContainerRegistriesInfiniteQuery = (search?: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createInfiniteQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const s = search?.() ?? "";
		const searchParam = s ? `&search[name]=${encodeURIComponent(s)}` : "";
		return {
			queryKey: containerRegistryKeys.infiniteList(wsId ?? "", s),
			// A new search re-keys this. Keep the previous results up until the
			// new ones land, rather than blanking the open picker.
			placeholderData: keepPreviousData,
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch container registries" },
			initialPageParam: 0,
			queryFn: async ({ pageParam }: { pageParam: number }): Promise<RepositoryPage> => {
				const response = await httpRequest<ListContainerRepositoriesResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry?page=${pageParam}&count=${REPOSITORY_PICKER_PAGE_SIZE}${searchParam}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return {
					repositories: response.data.repositories,
					totalCount: Number(response.headers.get("x-total-count") ?? 0),
					page: pageParam,
				};
			},
			getNextPageParam: (lastPage: RepositoryPage): number | undefined => {
				const loaded = (lastPage.page + 1) * REPOSITORY_PICKER_PAGE_SIZE;
				return loaded < lastPage.totalCount ? lastPage.page + 1 : undefined;
			},
		};
	});
};

export const useContainerRegistryInfoQuery = (id: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<GetContainerRepositoryInfoResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const repoId = id();
		return {
			queryKey: containerRegistryKeys.detail(wsId ?? "", repoId),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!repoId,
			meta: { errorMessage: "Failed to fetch repository info" },
			queryFn: async () => {
				const response = await httpRequest<GetContainerRepositoryInfoResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}`,
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

export const useContainerManifestsQuery = (repoId: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<ListContainerRepositoryManifestsResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const id = repoId();
		return {
			queryKey: containerRegistryKeys.manifests(wsId ?? "", id),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!id,
			meta: { errorMessage: "Failed to fetch manifests" },
			queryFn: async () => {
				const response = await httpRequest<ListContainerRepositoryManifestsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${id}/manifest`,
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

export const useContainerManifestDetailsQuery = (repoId: Accessor<string>, digestOrTag: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<GetContainerRepositoryManifestDetailsResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const id = repoId();
		const reference = digestOrTag();
		return {
			queryKey: containerRegistryKeys.manifestDetail(wsId ?? "", id, reference),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!id && !!reference,
			meta: { errorMessage: "Failed to fetch manifest details" },
			queryFn: async () => {
				const response = await httpRequest<GetContainerRepositoryManifestDetailsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${id}/manifest/${reference}`,
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

export const useContainerExposedPortsQuery = (repoId: Accessor<string>, digestOrTag: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<GetContainerRepositoryExposedPortsResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const id = repoId();
		const reference = digestOrTag();
		return {
			queryKey: containerRegistryKeys.ports(wsId ?? "", id, reference),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!id && !!reference,
			// Exposed ports are best-effort (only used to show the port list and,
			// later, prefill the deploy flow). They can't be computed for artifacts
			// or when the config blob isn't readable, so a failure must fail
			// silently — no error toast. Intentionally no `meta.errorMessage`, and
			// no retries on what is often a deterministic 4xx/5xx.
			retry: false,
			queryFn: async () => {
				const response = await httpRequest<GetContainerRepositoryExposedPortsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${id}/manifest/${reference}/exposed-ports`,
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

export const useContainerRegistryUsageQuery = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<GetContainerRegistryUsageResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		return {
			queryKey: containerRegistryKeys.usage(wsId ?? ""),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch registry usage" },
			queryFn: async () => {
				const response = await httpRequest<GetContainerRegistryUsageResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/usage`,
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

export const useContainerTagsQuery = (repoId: Accessor<string>, search?: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery<ListContainerRepositoryTagsResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const id = repoId();
		const s = search?.();
		return {
			queryKey: containerRegistryKeys.tags(wsId ?? "", id, s),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!id,
			meta: { errorMessage: "Failed to fetch tags" },
			initialData: { tags: [] } as ListContainerRepositoryTagsResponse,
			queryFn: async () => {
				const url = new URL(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${id}/tag`
				);
				if (s) url.searchParams.set("tag", s);
				const response = await httpRequest<ListContainerRepositoryTagsResponse>(url.toString(), {
					method: "GET",
				});

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return response.data;
			},
		};
	});
};
