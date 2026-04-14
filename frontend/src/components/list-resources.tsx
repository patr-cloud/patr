import { Show } from "solid-js";
import { createInfiniteQuery } from "@tanstack/solid-query";
import { useAuthState } from "~/hooks";
import { get, getResourceEndpoint, parseCamelCase } from "~/utils/func";
import { resourceKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";
import InputDropdownCheckbox from "./input-dropdown-checkbox";
import { MaybeAccessor } from "~/utils/types";

const PAGE_SIZE = 20;

type ResourcePage = {
	items: { id: string; name: string }[];
	totalCount: number;
	page: number;
};

const extractItems = (data: Record<string, { id: string; name: string }[]>) => {
	return (
		data.deployments ||
		data.runners ||
		data.repositories ||
		data.staticSites ||
		data.volumes ||
		data.databases ||
		data.secrets ||
		[]
	);
};

const ListResources = (props: {
	workspaceId: MaybeAccessor<string>;
	resourceType: MaybeAccessor<string>;
	selectedResources: MaybeAccessor<Set<string>>;
	toggleResource: (resourceId: string) => void;
}) => {
	const [authState] = useAuthState();

	const resourcesQuery = createInfiniteQuery(() => {
		const auth = authState();
		const wsId = get(props.workspaceId);
		const type = get(props.resourceType);
		const endpoint = type ? getResourceEndpoint(type) : undefined;
		return {
			queryKey: resourceKeys.list(wsId ?? "", type ?? ""),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!type && !!endpoint,
			meta: { errorMessage: `Failed to fetch ${type}` },
			initialPageParam: 0,
			queryFn: async ({ pageParam }: { pageParam: number }): Promise<ResourcePage> => {
				const response = await httpRequest<Record<string, { id: string; name: string }[]>>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/${endpoint}?page=${pageParam}&count=${PAGE_SIZE}`,
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
				const loaded = (lastPage.page + 1) * PAGE_SIZE;
				return loaded < lastPage.totalCount ? lastPage.page + 1 : undefined;
			},
		};
	});

	const allResources = () => resourcesQuery.data?.pages.flatMap((page) => page.items) ?? [];

	const getResourceTypeLabel = () => {
		if (!props.resourceType) return "Resources";
		return parseCamelCase(get(props.resourceType));
	};

	return (
		<Show
			when={allResources().length > 0 || resourcesQuery.isFetching}
			fallback={<span class="text-gray-400 text-sm">No {getResourceTypeLabel().toLowerCase()} found</span>}
		>
			<InputDropdownCheckbox
				onToggle={props.toggleResource}
				checked={() => Array.from(get(props.selectedResources))}
				placeholder={`Select ${getResourceTypeLabel()}`}
				options={() =>
					allResources().map((resource) => ({
						label: resource.name,
						value: resource.id,
					}))
				}
				onLoadMore={resourcesQuery.hasNextPage ? () => resourcesQuery.fetchNextPage() : undefined}
				isLoadingMore={() => resourcesQuery.isFetchingNextPage}
			/>
		</Show>
	);
};

export default ListResources;
