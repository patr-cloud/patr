import { Show } from "solid-js";
import { get, parseCamelCase } from "~/utils/func";
import { useWorkspaceResourcesQuery } from "~/hooks/fetch/resources";
import InputDropdownCheckbox from "./input-dropdown-checkbox";
import { MaybeAccessor } from "~/utils/types";

const ListResources = (props: {
	workspaceId: MaybeAccessor<string>;
	resourceType: MaybeAccessor<string>;
	selectedResources: MaybeAccessor<Set<string>>;
	toggleResource: (resourceId: string) => void;
}) => {
	const resourcesQuery = useWorkspaceResourcesQuery(
		() => get(props.workspaceId),
		() => get(props.resourceType)
	);

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
