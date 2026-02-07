import { createMemo, createSignal, Show, Suspense } from "solid-js";
import { InputDropdown, InputDropdownCheckBox, ListResources } from "~/components";
import useFetchPermissions from "~/hooks/use-fetch/use-fetch-permissions";
import { parsePermissionName, parseCamelCase, getResourceEndpoint } from "~/utils/func";
import { ResourcePermissionType } from "~/bindings/ResourcePermissionType";

interface PermissionSelectorProps {
	workspaceId: string;
	selectedPermissionIds: Set<string>;
	onPermissionChange: (permissions: Set<string>) => void;
	onPermissionsDataChange?: (data: { [key: string]: ResourcePermissionType }) => void;
}

const PermissionSelector = (props: PermissionSelectorProps) => {
	const [selectedResourceType, setSelectedResourceType] = createSignal<string>("");
	const [selectedResources, setSelectedResources] = createSignal<Set<string>>(new Set());
	const [includeExcludeMode, setIncludeExcludeMode] = createSignal<"all" | "include" | "exclude">("all");

	const [permissions] = useFetchPermissions(props.workspaceId);

	const togglePermissionId = (permissionId: string) => {
		const newSet = new Set(props.selectedPermissionIds);
		if (newSet.has(permissionId)) {
			newSet.delete(permissionId);
		} else {
			newSet.add(permissionId);
		}
		props.onPermissionChange(newSet);
		updatePermissionsData(newSet);
	};

	const toggleResource = (resourceId: string) => {
		const newSet = new Set(selectedResources());
		if (newSet.has(resourceId)) {
			newSet.delete(resourceId);
		} else {
			newSet.add(resourceId);
		}
		setSelectedResources(newSet);
		updatePermissionsData(props.selectedPermissionIds);
	};

	const updatePermissionsData = (permissionIds: Set<string>) => {
		if (!props.onPermissionsDataChange) return;

		const permissionsData: { [key: string]: ResourcePermissionType } = {};
		const mode = includeExcludeMode();

		permissionIds.forEach((permissionId) => {
			if (mode === "all") {
				permissionsData[permissionId] = {
					permissionType: "exclude",
					resources: [],
				};
			} else if (mode === "include" && selectedResources().size > 0) {
				permissionsData[permissionId] = {
					permissionType: "include",
					resources: Array.from(selectedResources()),
				};
			} else if (mode === "exclude" && selectedResources().size > 0) {
				permissionsData[permissionId] = {
					permissionType: "exclude",
					resources: Array.from(selectedResources()),
				};
			} else {
				permissionsData[permissionId] = {
					permissionType: "include",
					resources: [],
				};
			}
		});

		props.onPermissionsDataChange(permissionsData);
	};

	const permissionActions = createMemo(() => {
		return (permissions()?.permissions || []).filter((p) => {
			const parsed = parsePermissionName(p.name);
			return parsed.action !== "" ? parsed.resourceType === selectedResourceType() : null;
		});
	});

	return (
		<Suspense fallback={<div class="text-gray-400 text-sm">Loading permissions...</div>}>
			<div class="flex gap-3">
				{/* Column 1: Resource Types */}
				<div class="flex flex-col gap-3 w-full">
					<InputDropdown
						onSelect={(val) => {
							console.log(val);
							setSelectedResourceType(val);
							setIncludeExcludeMode("all");
							props.onPermissionChange(new Set<string>([]));
							setSelectedResources(new Set<string>([]));
						}}
						placeholder="Select Resource Type"
						value={selectedResourceType}
						options={Array.from(
							new Set(
								(permissions()?.permissions || []).map((p) => parsePermissionName(p.name).resourceType).filter((r) => r)
							)
						).map((resourceType) => ({
							label: parseCamelCase(resourceType),
							value: resourceType,
						}))}
					/>
				</div>

				{/* Column 2: Permission Actions */}
				<Show when={selectedResourceType() && permissionActions().length > 0}>
					<div class="flex flex-col gap-3 w-full">
						<InputDropdownCheckBox
							onToggle={(val) => togglePermissionId(val)}
							checked={() => Array.from(props.selectedPermissionIds)}
							placeholder={() =>
								Array.from(props.selectedPermissionIds)
									.map((s) => permissionActions().find((p) => p.id === s)?.name)
									.map((val) => (val ? parseCamelCase(parsePermissionName(val).action) : undefined))
									.join(", ") || "Select Permissions"
							}
							options={() =>
								permissionActions().map((p) => {
									const parsed = parsePermissionName(p.name);
									return {
										label: `${parseCamelCase(parsed.action)}`,
										value: p.id,
									};
								})
							}
						/>
					</div>
				</Show>

				{/* Column 3: Include/Exclude Mode */}
				<Show when={selectedResourceType() && getResourceEndpoint(selectedResourceType())}>
					<div class="flex flex-col gap-3 w-full">
						<InputDropdown
							onSelect={(val) => {
								setIncludeExcludeMode(val as "all" | "include" | "exclude");
								if (val === "all") {
									setSelectedResources(new Set<string>([]));
								}
								updatePermissionsData(props.selectedPermissionIds);
							}}
							placeholder="Select Include/Exclude Mode"
							value={includeExcludeMode}
							options={[
								{
									label: `All ${parseCamelCase(selectedResourceType())}(s)`,
									value: "all",
								},
								{
									label: `Include Specific ${parseCamelCase(selectedResourceType())}(s)`,
									value: "include",
								},
								{
									label: `Exclude Specific ${parseCamelCase(selectedResourceType())}(s)`,
									value: "exclude",
								},
							]}
						/>
					</div>
				</Show>

				{/* Column 4: List of Resources */}
				<Show
					when={selectedResourceType() && includeExcludeMode() !== "all" && getResourceEndpoint(selectedResourceType())}
				>
					<div class="flex flex-col gap-3 w-full">
						<ListResources
							workspaceId={props.workspaceId}
							resourceType={selectedResourceType()}
							selectedResources={selectedResources()}
							toggleResource={toggleResource}
						/>
					</div>
				</Show>
			</div>
		</Suspense>
	);
};

export default PermissionSelector;
