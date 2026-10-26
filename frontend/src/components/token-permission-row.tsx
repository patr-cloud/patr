import { createSignal, Show, untrack } from "solid-js";
import { FiX } from "solid-icons/fi";
import Input from "./input";
import InputDropdown, { InputDropdownOption } from "./input-dropdown";
import ListResources from "./list-resources";
import { Scope, scopableResourceType, scopeResources } from "~/utils/scope";
import type { PermissionInfo } from "./token-permission-editor";

/** The scope choices for a permission, narrowed to what it can actually be scoped to. */
export const scopeOptions = (resourceType: string | undefined): InputDropdownOption[] => [
	{ label: "Entire workspace", value: "workspace" },
	...(resourceType ? [{ label: "Specific resources", value: "resources" }] : []),
];

interface TokenPermissionRowProps {
	workspaceId: string;
	permissionId: string;
	scope: Scope;
	catalogue: Map<string, PermissionInfo>;
	/** The grouped permission options, shared with the add row. */
	options: InputDropdownOption[];
	/** Resolves a scoped resource id to a display name. */
	resourceName: (resourceId: string) => string;
	editing: boolean;
	onEdit: () => void;
	/** Commits the edit, with the row's permission and the resource ids it now covers. */
	onDone: (permissionId: string, scopeIds: string[]) => void;
	onCancel: () => void;
	onRemove: () => void;
}

/**
 * One granted permission in the token editor's table. Read-only until Edit,
 * which swaps the cells for dropdowns at the same row height.
 */
const TokenPermissionRow = (props: TokenPermissionRowProps) => {
	const permission = () => props.catalogue.get(props.permissionId);
	const label = () => permission()?.label ?? props.permissionId;

	const scopeText = () =>
		props.scope.scopeType === "workspace"
			? "All resources"
			: props.scope.resources.map(props.resourceName).join(", ") || "None";

	return (
		<li class="grid grid-cols-1 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1.5fr)_auto] items-center gap-2 sm:gap-3 py-xs min-h-12 border-b border-border-color last:border-0">
			<Show
				when={props.editing}
				fallback={
					<>
						<span class="text-sm truncate">
							<span class="text-white">{label()}</span>
							<span class="text-grey">&nbsp;·&nbsp;{permission()?.typeLabel}</span>
						</span>
						<span class="text-grey text-sm truncate">
							{props.scope.scopeType === "workspace" ? "Entire workspace" : "Specific resources"}
						</span>
						<span class="text-grey text-sm truncate" title={scopeText()}>
							{scopeText()}
						</span>
						<div class="flex items-center justify-end gap-3">
							<button
								type="button"
								onClick={() => props.onEdit()}
								class="text-grey hover:text-white text-sm transition-colors cursor-pointer"
							>
								Edit
							</button>
							<button
								type="button"
								aria-label={`Remove ${label()}`}
								onClick={() => props.onRemove()}
								class="text-grey hover:text-error transition-colors cursor-pointer"
							>
								<FiX size={14} />
							</button>
						</div>
					</>
				}
			>
				<RowEditor {...props} />
			</Show>
		</li>
	);
};

/** The edit-mode cells of a row. Holds the draft until Done. */
const RowEditor = (props: TokenPermissionRowProps) => {
	// The draft is seeded once, when Edit is pressed; later prop changes don't reset it.
	const initial = untrack(() => ({
		permissionId: props.permissionId,
		scopeType: props.scope.scopeType,
		resources: props.scope.scopeType === "resources" ? props.scope.resources : [],
	}));
	const [permissionId, setPermissionId] = createSignal(initial.permissionId);
	const [scopeType, setScopeType] = createSignal(initial.scopeType);
	const [resources, setResources] = createSignal(initial.resources);

	const resourceType = () => scopableResourceType([props.catalogue.get(permissionId())?.resourceType ?? ""]);

	// A permission on a different resource type can't keep the old resources.
	const selectPermission = (id: string) => {
		const before = resourceType();
		setPermissionId(id);
		if (resourceType() !== before) {
			setScopeType("workspace");
			setResources([]);
		}
	};

	const toggleResource = (resourceId: string) =>
		setResources((current) =>
			current.includes(resourceId) ? current.filter((id) => id !== resourceId) : [...current, resourceId]
		);

	const draftScope = (): Scope =>
		scopeType() === "workspace" ? { scopeType: "workspace" } : { scopeType: "resources", resources: resources() };

	const unchanged = () =>
		permissionId() === initial.permissionId &&
		scopeType() === initial.scopeType &&
		JSON.stringify([...resources()].sort()) === JSON.stringify([...initial.resources].sort());

	// Done with nothing changed leaves the grants untouched, so a merged row's
	// extra resource ids aren't collapsed by a no-op edit.
	const done = () => {
		if (unchanged()) props.onCancel();
		else props.onDone(permissionId(), scopeResources(draftScope(), props.workspaceId));
	};

	return (
		<>
			<InputDropdown
				styleVariant="medium"
				options={props.options}
				value={permissionId}
				onSelect={selectPermission}
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
			<div class="flex items-center justify-end gap-3">
				<button
					type="button"
					onClick={done}
					disabled={scopeType() === "resources" && resources().length === 0}
					class="text-primary text-sm transition-colors cursor-pointer disabled:text-disabled disabled:cursor-not-allowed"
				>
					Done
				</button>
				<button
					type="button"
					aria-label="Cancel editing"
					onClick={() => props.onCancel()}
					class="text-grey hover:text-white transition-colors cursor-pointer"
				>
					<FiX size={14} />
				</button>
			</div>
		</>
	);
};

export default TokenPermissionRow;
