import { createEffect, createMemo, createSignal, For, JSX, on, Show, Suspense, untrack } from "solid-js";
import { Checkbox } from "~/components";
import { usePermissionsQuery } from "~/hooks/fetch";
import { parseCamelCase, parsePermissionName, workspaceLevelResourceTypes } from "~/utils/func";

/**
 * Checkbox cards sitting alongside the pills. The transparent border is always
 * present so that hovering changes only its colour and nothing shifts.
 */
const CHECKBOX_CARD_CLASS =
	"w-full min-w-0 px-3 py-2 rounded-xs bg-secondary border border-transparent hover:border-grey";

interface PermissionPickerProps {
	/** Additional classes for the container */
	class?: string;
	workspaceId: string;
	/** The IDs of the permissions the role currently grants. */
	selected: ReadonlySet<string>;
	/**
	 * Called with the complete next permission set. The full set (rather than a
	 * partial merge) so that removing a permission can be expressed.
	 */
	onChange: (next: Set<string>) => void;
	/** Render read-only: checkboxes disabled, nothing toggleable. */
	disabled?: boolean;
	/**
	 * Changes when the permissions come from the server — on load, and on the
	 * refetch after a save. Granted-first ordering is recomputed only then, so the
	 * columns hold still while you are editing instead of re-sorting under the
	 * cursor on every toggle.
	 */
	sortToken?: number;
}

/** The frozen display order of the columns. */
type SortSnapshot = {
	resourceTypes: string[];
	actionsByResourceType: Record<string, string[]>;
};

const Column = (props: { heading: string; children: JSX.Element }) => (
	<div class="flex flex-col gap-2 min-w-0 flex-1">
		<div class="text-white text-sm font-medium">{props.heading}</div>
		<div class="flex flex-col gap-2 border border-border-color rounded-xs bg-secondary-light p-2 h-72 overflow-y-auto">
			{props.children}
		</div>
	</div>
);

/**
 * A selectable card. The border marks which card is being viewed; the badge
 * marks that the role already grants something here, so the two signals stay
 * independent — a card can be granted without being selected, and vice versa.
 */
const Pill = (props: {
	label: string;
	selected: boolean;
	/** Shown when the role grants something under this card. */
	badge?: string;
	disabled?: boolean;
	onClick: () => void;
}) => (
	<button
		type="button"
		disabled={props.disabled}
		aria-pressed={props.selected}
		onClick={() => props.onClick()}
		class={`w-full flex items-center justify-between gap-2 px-3 py-2 rounded-xs text-sm
			transition-colors cursor-pointer border disabled:cursor-not-allowed disabled:opacity-50
			min-h-8.75 bg-secondary ${
				props.selected ? "border-primary text-white" : "border-transparent text-grey hover:border-grey"
			}`}
	>
		<span class="truncate text-left">{props.label}</span>
		<Show when={props.badge}>
			<span class="shrink-0 rounded-full bg-primary/20 text-primary text-xs px-1.5 py-0.5">{props.badge}</span>
		</Show>
	</button>
);

/** Placeholder shown in a column that has nothing to offer for the current selection. */
const ColumnHint = (props: { children: JSX.Element }) => (
	<span class="text-grey text-xs italic px-1">{props.children}</span>
);

/**
 * Two-column editor for a role's permissions: resource type, and the actions
 * the role grants on it. A role is a flat list of permissions — where it
 * applies is decided when the role is assigned, not here.
 *
 * Column 1 is a viewport for the actioned types (the pills drill into column
 * 2 and grant nothing on their own); workspace-level types (`viewRoles`,
 * `modifyRoles`, `editWorkspace`) carry no actions, so those render as
 * checkboxes and toggle directly. Every action in column 2 is a checkbox —
 * with no scoping in the role there is nothing to configure beyond on or off.
 *
 * Edits are local. The parent persists them.
 */
const PermissionPicker = (props: PermissionPickerProps) => {
	const [selectedResourceType, setSelectedResourceType] = createSignal<string>("");

	const permissionsQuery = usePermissionsQuery(() => props.workspaceId);
	const permissions = () => permissionsQuery.data?.permissions ?? [];

	const resourceTypes = createMemo(() =>
		Array.from(
			new Set(permissions().map((permission) => parsePermissionName(permission.name).resourceType))
		).filter((resourceType) => resourceType)
	);

	const permissionIdFor = (resourceType: string, action: string) =>
		permissions().find((permission) => {
			const parsed = parsePermissionName(permission.name);
			return parsed.resourceType === resourceType && parsed.permission === action;
		})?.id;

	/** How many of a resource type's actions the role currently grants. */
	const grantedActionCount = (resourceType: string) =>
		permissions().filter((permission) => {
			const parsed = parsePermissionName(permission.name);
			return parsed.resourceType === resourceType && props.selected.has(permission.id);
		}).length;

	const isActionGranted = (resourceType: string, action: string) => {
		const permissionId = permissionIdFor(resourceType, action);
		return !!permissionId && props.selected.has(permissionId);
	};

	/** Workspace-level permissions have no action — they are just on or off. */
	const workspaceLevelPermissionId = (resourceType: string) =>
		permissions().find((permission) => parsePermissionName(permission.name).resourceType === resourceType)?.id;

	const isWorkspaceLevelGranted = (resourceType: string) => {
		const permissionId = workspaceLevelPermissionId(resourceType);
		return !!permissionId && props.selected.has(permissionId);
	};

	const actionsOf = (resourceType: string) =>
		permissions()
			.filter((permission) => parsePermissionName(permission.name).resourceType === resourceType)
			.map((permission) => parsePermissionName(permission.name).permission)
			.filter((action) => action);

	/**
	 * Rank both columns granted-first, most-granted-first.
	 *
	 * Pills and checkboxes are interleaved rather than kept in separate blocks so
	 * that everything the role grants floats to the top of its column, whichever
	 * kind of card it is. Array.sort is stable, so equal scores keep the order the
	 * permission list gave them — which leaves the workspace-level types at the
	 * bottom while ungranted, since they come last in that list.
	 */
	const computeSortSnapshot = (): SortSnapshot => {
		const rankedTypes = resourceTypes().map((resourceType) => ({
			resourceType,
			granted: workspaceLevelResourceTypes.has(resourceType)
				? isWorkspaceLevelGranted(resourceType)
					? 1
					: 0
				: grantedActionCount(resourceType),
		}));
		rankedTypes.sort((a, b) => b.granted - a.granted);

		const actionsByResourceType: Record<string, string[]> = {};
		for (const { resourceType } of rankedTypes) {
			const rankedActions = actionsOf(resourceType).map((action) => ({
				action,
				granted: isActionGranted(resourceType, action) ? 1 : 0,
			}));
			rankedActions.sort((a, b) => b.granted - a.granted);
			actionsByResourceType[resourceType] = rankedActions.map((entry) => entry.action);
		}

		return {
			resourceTypes: rankedTypes.map((entry) => entry.resourceType),
			actionsByResourceType,
		};
	};

	const [sortSnapshot, setSortSnapshot] = createSignal<SortSnapshot>({
		resourceTypes: [],
		actionsByResourceType: {},
	});

	// Re-rank only when the permission catalogue finishes loading or the server
	// hands back a new set of grants. `untrack` keeps the ranking itself from
	// subscribing to `selected`, so editing never re-orders the columns.
	createEffect(
		on(
			() => [props.sortToken, permissions().length] as const,
			() => setSortSnapshot(untrack(computeSortSnapshot))
		)
	);

	const resourceTypeCards = createMemo(() => {
		const ordered = sortSnapshot().resourceTypes;
		// Fall back to the unsorted list until the first snapshot is taken, and
		// append anything the snapshot predates so no card can go missing.
		const known = new Set(ordered);
		return [...ordered, ...resourceTypes().filter((resourceType) => !known.has(resourceType))].map(
			(resourceType) => ({
				resourceType,
				isWorkspaceLevel: workspaceLevelResourceTypes.has(resourceType),
			})
		);
	});

	const actions = createMemo(() => {
		const resourceType = selectedResourceType();
		if (!resourceType) return [];

		const ordered = sortSnapshot().actionsByResourceType[resourceType] ?? [];
		const known = new Set(ordered);
		return [...ordered, ...actionsOf(resourceType).filter((action) => !known.has(action))];
	});

	/** Badge for a resource type: how many of its actions the role grants. */
	const resourceTypeBadge = (resourceType: string) => {
		const granted = grantedActionCount(resourceType);
		return granted > 0 ? String(granted) : undefined;
	};

	const togglePermission = (permissionId: string | undefined) => {
		if (!permissionId || props.disabled) return;

		const next = new Set(props.selected);
		if (next.has(permissionId)) next.delete(permissionId);
		else next.add(permissionId);
		props.onChange(next);
	};

	const isWorkspaceLevelSelected = createMemo(() => workspaceLevelResourceTypes.has(selectedResourceType()));

	return (
		<Suspense fallback={<div class="text-gray-400 text-sm">Loading permissions...</div>}>
			<div class={`flex flex-col md:flex-row gap-4 ${props.class || ""}`}>
				<Column heading="Resource Type">
					<For each={resourceTypeCards()}>
						{(card) => (
							<Show
								when={!card.isWorkspaceLevel}
								fallback={
									/*
									  Workspace-level types carry no actions, so there is nothing
									  to drill into — a checkbox reads as "switch this on" where
									  the pills read as "drill into this".

									  The Checkbox owns the click: it renders a <label>, so wrapping
									  it in a clickable parent would fire that parent twice — once
									  for the real click and once for the synthetic click the label
									  dispatches on its input.
									*/
									<Checkbox
										size="sm"
										class={CHECKBOX_CARD_CLASS}
										label={parseCamelCase(card.resourceType)}
										checked={() => isWorkspaceLevelGranted(card.resourceType)}
										disabled={props.disabled}
										onChange={() => {
											setSelectedResourceType(card.resourceType);
											togglePermission(workspaceLevelPermissionId(card.resourceType));
										}}
									/>
								}
							>
								<Pill
									label={parseCamelCase(card.resourceType)}
									badge={resourceTypeBadge(card.resourceType)}
									selected={selectedResourceType() === card.resourceType}
									onClick={() => setSelectedResourceType(card.resourceType)}
								/>
							</Show>
						)}
					</For>
				</Column>

				<Column heading="Actions">
					<Show when={selectedResourceType()} fallback={<ColumnHint>Select a resource type</ColumnHint>}>
						<Show
							when={!isWorkspaceLevelSelected()}
							fallback={<ColumnHint>This permission has no actions</ColumnHint>}
						>
							<For each={actions()}>
								{(action) => (
									<Checkbox
										size="sm"
										class={CHECKBOX_CARD_CLASS}
										label={parseCamelCase(action)}
										checked={() => isActionGranted(selectedResourceType(), action)}
										disabled={props.disabled}
										onChange={() =>
											togglePermission(permissionIdFor(selectedResourceType(), action))
										}
									/>
								)}
							</For>
						</Show>
					</Show>
				</Column>
			</div>
		</Suspense>
	);
};

export default PermissionPicker;
