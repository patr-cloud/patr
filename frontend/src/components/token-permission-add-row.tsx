import { createSignal, Show } from "solid-js";
import Button from "./button";
import { ButtonVariant } from "~/utils/color";
import Input from "./input";
import InputDropdown, { InputDropdownOption } from "./input-dropdown";
import InputDropdownCheckbox from "./input-dropdown-checkbox";
import ListResources from "./list-resources";
import { scopeOptions } from "./token-permission-row";
import { Scope, scopableResourceType } from "~/utils/scope";
import type { PermissionInfo } from "./token-permission-editor";

interface TokenPermissionAddRowProps {
	workspaceId: string;
	catalogue: Map<string, PermissionInfo>;
	/** The grouped permission options, shared with the table rows. */
	options: InputDropdownOption[];
	/** Adds every picked permission at the picked scope (resource ids, or the workspace's own id). */
	onAdd: (permissionIds: string[], scopeIds: string[]) => void;
}

/**
 * The token editor's add row: pick one or more permissions, where they apply,
 * and add them to the table in one go.
 */
const TokenPermissionAddRow = (props: TokenPermissionAddRowProps) => {
	const [permissionIds, setPermissionIds] = createSignal<string[]>([]);
	const [scopeType, setScopeType] = createSignal<Scope["scopeType"]>("workspace");
	const [resources, setResources] = createSignal<string[]>([]);

	// Specific resources only make sense when every picked permission acts on
	// the same listable resource type.
	const resourceType = () =>
		scopableResourceType(permissionIds().map((id) => props.catalogue.get(id)?.resourceType ?? ""));

	// The scope resets whenever the shared resource type changes, so resources
	// of one type never carry over to permissions on another.
	const togglePermission = (id: string) => {
		const before = resourceType();
		setPermissionIds((current) => (current.includes(id) ? current.filter((p) => p !== id) : [...current, id]));
		if (resourceType() !== before) {
			setScopeType("workspace");
			setResources([]);
		}
	};

	const toggleResource = (resourceId: string) =>
		setResources((current) =>
			current.includes(resourceId) ? current.filter((id) => id !== resourceId) : [...current, resourceId]
		);

	const canAdd = () => permissionIds().length > 0 && (scopeType() === "workspace" || resources().length > 0);

	const add = () => {
		props.onAdd(permissionIds(), scopeType() === "workspace" ? [props.workspaceId] : resources());
		setPermissionIds([]);
		setScopeType("workspace");
		setResources([]);
	};

	return (
		<div class="grid grid-cols-1 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1.5fr)_auto] items-center gap-2 sm:gap-3 p-sm border border-border-color rounded-xs">
			<InputDropdownCheckbox
				styleVariant="medium"
				placeholder={
					permissionIds().length > 0
						? `${permissionIds().length} permission${permissionIds().length === 1 ? "" : "s"} selected`
						: "Select permissions"
				}
				options={props.options}
				checked={permissionIds}
				onToggle={togglePermission}
			/>
			<InputDropdown
				styleVariant="medium"
				options={scopeOptions(resourceType())}
				value={scopeType}
				onSelect={(value) => setScopeType(value as Scope["scopeType"])}
			/>
			<Show
				when={scopeType() === "resources" && resourceType()}
				fallback={<Input styleVariant="medium" value="All resources" disabled />}
			>
				{(type) => (
					<ListResources
						styleVariant="medium"
						workspaceId={props.workspaceId}
						resourceType={type()}
						selectedResources={new Set(resources())}
						toggleResource={toggleResource}
						placeholder={resources().length > 0 ? `${resources().length} selected` : undefined}
					/>
				)}
			</Show>
			<Button type="button" variant={ButtonVariant.Contained} disabled={!canAdd()} onClick={add}>
				Add
			</Button>
		</div>
	);
};

export default TokenPermissionAddRow;
