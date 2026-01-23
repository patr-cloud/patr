import { createMemo, createResource, Show, Suspense } from "solid-js";
import { useAuthState } from "~/hooks";
import { get, getResourceEndpoint, parseCamelCase } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";
import InputDropdownCheckbox from "./input-dropdown-checkbox";
import { MaybeAccessor } from "~/utils/types";

const ListResources = (props: {
	workspaceId: MaybeAccessor<string>;
	resourceType: MaybeAccessor<string>;
	selectedResources: MaybeAccessor<Set<string>>;
	toggleResource: (resourceId: string) => void;
}) => {
	const [authState] = useAuthState();

	const fetchParams = createMemo(() => {
		return [authState(), get(props.workspaceId), get(props.resourceType)] as const;
	});

	const [resources] = createResource(fetchParams, async ([auth, wsId, type]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || !type) {
			return null;
		}

		const endpoint = getResourceEndpoint(type);
		console.log("Fetching resources for type:", type, "using endpoint:", endpoint);
		if (!endpoint) {
			return null;
		}

		const response = await httpRequest<any>(`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/${endpoint}`, {
			method: "GET",
			headers: {
				"Content-Type": "application/json",
				Authorization: `Bearer ${auth.accessToken}`,
			},
		});

		if (!response.ok) {
			console.error(`Failed to fetch ${type}:`, response.data.error);
			return null;
		}

		console.log("Fetched data for", type, ":", response.data);
		return { data: response.data, type }; // Include type to track which resource this data is for
	});
	// Helper to get the resource list from the response
	const getResourceList = () => {
		const resourceData = resources();
		if (!resourceData) return [];

		// Check if the data is for the current resource type
		if (resourceData.type !== get(props.resourceType)) {
			console.log("mismatch", get(props.resourceType), resourceData.type);
			return []; // Return empty if data doesn't match current type
		}

		const data = resourceData.data;
		if (!data) return [];

		// Handle different response structures
		if (data.deployments) return data.deployments;
		if (data.runners) return data.runners;
		if (data.repositories) return data.repositories;
		if (data.staticSites) return data.staticSites;
		if (data.volumes) return data.volumes;
		if (data.databases) return data.databases;
		if (data.secrets) return data.secrets;

		return [];
	};

	// Get resource type label
	const getResourceTypeLabel = () => {
		if (!props.resourceType) return "Resources";
		return parseCamelCase(get(props.resourceType));
	};

	return (
		<Suspense fallback={<div>Loading {getResourceTypeLabel()}...</div>}>
			<Show
				when={getResourceList().length > 0}
				fallback={<span class="text-gray-400 text-sm">No {getResourceTypeLabel().toLowerCase()} found</span>}
			>
				<InputDropdownCheckbox
					onToggle={props.toggleResource}
					checked={() => Array.from(get(props.selectedResources))}
					placeholder={`Select ${getResourceTypeLabel()}`}
					options={() =>
						getResourceList().map((resource: any) => ({
							label: resource.name,
							value: resource.id,
						}))
					}
				/>
			</Show>
		</Suspense>
	);
};

export default ListResources;
