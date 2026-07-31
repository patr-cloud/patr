import { createInfiniteQuery, createQuery } from "@tanstack/solid-query";
import { Accessor } from "solid-js";
import { GetResourcesInfoRequest } from "~/bindings/GetResourcesInfoRequest";
import { GetResourcesInfoResponse } from "~/bindings/GetResourcesInfoResponse";
import { ResourceInfo } from "~/bindings/ResourceInfo";

import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { resourceKeys } from "~/hooks/query-keys";
import { getResourceEndpoint } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";

/** Resources are listed 20 at a time. */
export const RESOURCE_PAGE_SIZE = 20;

/** A single resource as returned by the per-type list endpoints. */
export type ListedResource = { id: string; name: string };

/** Loosely-typed list item — each endpoint's array carries its own shape. */
type RawListItem = { id: string; name?: string; subDomain?: string; path?: string };

type ResourcePage = {
	items: ListedResource[];
	totalCount: number;
	page: number;
};

/**
 * Each resource type's list endpoint names its array differently, and not all
 * of them return a ready-made `name`. The first array-shaped key present wins;
 * domains already carry their full `name.tld`, while managed URLs have no name
 * and get a best-effort label composed from what the list returns.
 */
const extractItems = (data: Record<string, RawListItem[]>): ListedResource[] => {
	const named =
		data.deployments ||
		data.runners ||
		data.repositories ||
		data.staticSites ||
		data.volumes ||
		data.databases ||
		data.secrets ||
		data.domains;
	if (named) {
		return named.map((r) => ({ id: r.id, name: r.name ?? "" }));
	}

	// Managed URLs return no `name`, and the domain itself isn't in the response
	// (only its id), so the label is the subdomain + path — apex (`@`) drops to
	// just the path.
	if (data.urls) {
		return data.urls.map((u) => ({
			id: u.id,
			name: (u.subDomain && u.subDomain !== "@" ? u.subDomain : "") + (u.path ?? ""),
		}));
	}

	return [];
};

/**
 * Paginated list of a workspace's resources of one type.
 *
 * Shared by the resource picker dropdown (`ListResources`) and the permission
 * matrix, which render the same data differently — the query, paging and cache
 * key live here so both stay in step.
 */
export const useWorkspaceResourcesQuery = (
	workspaceId: Accessor<string | undefined>,
	resourceType: Accessor<string | undefined>
) => {
	const [authState] = useAuthState();

	return createInfiniteQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const type = resourceType();
		const endpoint = type ? getResourceEndpoint(type) : undefined;
		return {
			queryKey: resourceKeys.list(wsId ?? "", type ?? ""),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!type && !!endpoint,
			meta: { errorMessage: `Failed to fetch ${type}` },
			initialPageParam: 0,
			queryFn: async ({ pageParam }: { pageParam: number }): Promise<ResourcePage> => {
				const response = await httpRequest<Record<string, RawListItem[]>>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/${endpoint}?page=${pageParam}&count=${RESOURCE_PAGE_SIZE}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				const totalCount = Number(response.headers.get("x-total-count") ?? 0);
				return {
					items: extractItems(response.data),
					totalCount,
					page: pageParam,
				};
			},
			getNextPageParam: (lastPage: ResourcePage): number | undefined => {
				const loaded = (lastPage.page + 1) * RESOURCE_PAGE_SIZE;
				return loaded < lastPage.totalCount ? lastPage.page + 1 : undefined;
			},
		};
	});
};

/**
 * Resolves a batch of resource IDs into their names and resource types.
 *
 * Permissions only store resource IDs, so anything that needs to display them
 * (the role permissions table, for instance) has to resolve them first. The IDs
 * are sorted before use so that the same set of resources — in any order — hits
 * the same cache entry.
 *
 * IDs that don't resolve come back from the API with `null` fields rather than
 * being omitted (a deleted resource, or one whose type has no name), so the map
 * always has an entry for every ID that was asked for. Callers should fall back
 * to showing the raw ID in that case.
 */
const useResourcesInfoQuery = (resourceIds: Accessor<string[]>) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();

	return createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const ids = [...resourceIds()].sort();
		return {
			queryKey: resourceKeys.info(wsId ?? "", ids),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && ids.length > 0,
			meta: { errorMessage: "Failed to fetch resource details" },
			queryFn: async () => {
				const body: GetResourcesInfoRequest = { resourceIds: ids };
				const response = await httpRequest<GetResourcesInfoResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/resources-info`,
					{ method: "POST", body: JSON.stringify(body) }
				);

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return new Map<string, ResourceInfo>(
					(response.data.resources || []).map(({ id, ...info }) => [id, info])
				);
			},
		};
	});
};

export default useResourcesInfoQuery;
