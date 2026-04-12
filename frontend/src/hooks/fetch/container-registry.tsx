import { createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import {
	GetContainerRepositoryInfoResponse,
	ListContainerRepositoriesResponse,
	ListContainerRepositoryManifestsResponse,
	ListContainerRepositoryTagsResponse,
} from "~/bindings";
import { useToast } from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { containerRegistryKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";

export const useContainerRegistriesQuery = (
	page: Accessor<string | undefined>,
	count: Accessor<string | undefined>
) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const p = page();
		const c = count();
		return {
			queryKey: containerRegistryKeys.list(wsId ?? "", p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
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
					toast("Failed to fetch container registries", "error");
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

export const useContainerRegistryInfoQuery = (id: Accessor<string>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	return createQuery<GetContainerRepositoryInfoResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const repoId = id();
		return {
			queryKey: containerRegistryKeys.detail(wsId ?? "", repoId),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!repoId,
			queryFn: async () => {
				const response = await httpRequest<GetContainerRepositoryInfoResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					toast("Failed to fetch repository info", "error");
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
	const toast = useToast();

	return createQuery<ListContainerRepositoryManifestsResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const id = repoId();
		return {
			queryKey: containerRegistryKeys.manifests(wsId ?? "", id),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!id,
			queryFn: async () => {
				const response = await httpRequest<ListContainerRepositoryManifestsResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${id}/manifest`,
					{ method: "GET" }
				);

				if (!response.ok) {
					toast("Failed to fetch manifests", "error");
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
	const toast = useToast();

	return createQuery<ListContainerRepositoryTagsResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const id = repoId();
		const s = search?.();
		return {
			queryKey: containerRegistryKeys.tags(wsId ?? "", id, s),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!id,
			initialData: { tags: [] } as ListContainerRepositoryTagsResponse,
			placeholderData: (prev: ListContainerRepositoryTagsResponse | undefined) => prev,
			queryFn: async () => {
				const url = new URL(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${id}/tag`
				);
				if (s) url.searchParams.set("tag", s);
				const response = await httpRequest<ListContainerRepositoryTagsResponse>(url.toString(), {
					method: "GET",
				});

				if (!response.ok) {
					toast("Failed to fetch tags", "error");
					throw new Error(response.data.error);
				}

				return response.data;
			},
		};
	});
};
