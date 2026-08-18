import { createEffect, createMemo, createSignal, For, JSX, on, Show, Suspense, untrack } from "solid-js";
import { Button, ButtonVariant, Checkbox, LoadingSpinner } from "~/components";
import { ResourcePermissionType } from "~/bindings/ResourcePermissionType";
import { usePermissionsQuery } from "~/hooks/fetch";
import { useWorkspaceResourcesQuery } from "~/hooks/fetch/resources";
import { getResourceEndpoint, parseCamelCase, parsePermissionName, workspaceLevelResourceTypes } from "~/utils/func";

/** How a permission is scoped to the resources of its type. */
type ScopeMode = "all" | "include" | "exclude";

/** Actions that aren't tied to an existing resource, so can't be scoped to one. */
const UNSCOPABLE_ACTIONS = ["create", "add"];

/**
 * Checkbox cards sitting alongside the pills. The transparent border is always
 * present so that hovering changes only its colour and nothing shifts.
 */
const CHECKBOX_CARD_CLASS =
	"w-full min-w-0 px-3 py-2 rounded-xs bg-secondary border border-transparent hover:border-grey";

/** Resource rows in the third column — same hover treatment, no card background. */
const CHECKBOX_ROW_CLASS = "w-full min-w-0 px-2 py-1.5 rounded-xs border border-transparent hover:border-grey";

interface PermissionMatrixProps {
	/** Additional classes for the container */
	class?: string;
	workspaceId: string;
	/** The permissions currently granted, keyed by permission ID. */
	permissionsData: { [key: string]: ResourcePermissionType };
	/**
	 * Called with the complete next permissions map. The full map (rather than a
	 * partial merge) so that removing a permission can be expressed.
	 */
	onChange: (next: { [key: string]: ResourcePermissionType }) => void;
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
 * One segment of the scope switch. Labels are kept short so three fit across a
 * column; the full phrasing lives in the tooltip and in the heading beneath.
 */
const ScopeTab = (props: { label: string; title: string; selected: boolean; onClick: () => void }) => (
	<button
		type="button"
		title={props.title}
		aria-pressed={props.selected}
		onClick={() => props.onClick()}
		class={`flex-1 min-w-0 px-2 py-1.5 rounded-xs text-xs truncate border transition-colors
			cursor-pointer ${props.selected ? "border-primary text-white" : "border-transparent text-grey hover:border-grey"}`}
	>
		{props.label}
	</button>
);

/** Small uppercase heading above the resource list. */
const ListHeading = (props: { children: JSX.Element }) => (
	<span class="text-grey text-xxs uppercase tracking-wider px-1">{props.children}</span>
);

/**
 * Three-column editor for a role's permissions: resource type, action, and the
 * resources the permission applies to.
 *
 * Columns 1 and 2 are a viewport — they choose which `resourceType::action`
 * permission is being inspected and grant nothing on their own. The grant is
 * made in column 3, except for workspace-level types (`viewRoles`,
 * `modifyRoles`, `editWorkspace`), which carry no actions and no scoping at
 * all: those are rendered as checkboxes in column 1 and toggled directly.
 *
 * Edits are local. The parent persists them.
 */
const PermissionMatrix = (props: PermissionMatrixProps) => {
	const [selectedResourceType, setSelectedResourceType] = createSignal<string>("");
	const [selectedAction, setSelectedAction] = createSignal<string>("");

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
			return parsed.resourceType === resourceType && !!props.permissionsData[permission.id];
		}).length;

	const isActionGranted = (resourceType: string, action: string) => {
		const permissionId = permissionIdFor(resourceType, action);
		return !!permissionId && !!props.permissionsData[permissionId];
	};

	/** Workspace-level permissions have no action and no scope — they are just on or off. */
	const workspaceLevelPermissionId = (resourceType: string) =>
		permissions().find((permission) => parsePermissionName(permission.name).resourceType === resourceType)?.id;

	const isWorkspaceLevelGranted = (resourceType: string) => {
		const permissionId = workspaceLevelPermissionId(resourceType);
		return !!permissionId && !!props.permissionsData[permissionId];
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
	// subscribing to `permissionsData`, so editing never re-orders the columns.
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

	/** The permission ID for the currently selected resource type + action. */
	const currentPermissionId = createMemo(() => {
		const resourceType = selectedResourceType();
		const action = selectedAction();
		if (!resourceType || !action) return undefined;
		return permissionIdFor(resourceType, action);
	});

	const currentEntry = () => {
		const permissionId = currentPermissionId();
		return permissionId ? props.permissionsData[permissionId] : undefined;
	};

	/**
	 * Badge for a resource type: how many of its actions the role grants.
	 *
	 * A count of resources would be misleading here — an "All resources" grant
	 * stores no IDs, so a type could be granted and still show zero.
	 */
	const resourceTypeBadge = (resourceType: string) => {
		const granted = grantedActionCount(resourceType);
		return granted > 0 ? String(granted) : undefined;
	};

	/**
	 * Badge for an action: how many resources it is scoped to, or "All" when it
	 * applies to every resource of the type. Absent when the action isn't granted.
	 */
	const actionBadge = (action: string) => {
		const permissionId = permissionIdFor(selectedResourceType(), action);
		const entry = permissionId ? props.permissionsData[permissionId] : undefined;
		if (!entry) return undefined;
		return entry.resources.length === 0 ? "All" : String(entry.resources.length);
	};

	/**
	 * Whether a permission can be narrowed to specific resources. Actions that
	 * bring a resource into existence have nothing to point at, and a type with no
	 * list endpoint has nothing to enumerate.
	 */
	const isActionScopable = (resourceType: string, action: string) => {
		if (!resourceType || !action) return false;
		if (!getResourceEndpoint(resourceType)) return false;
		return !UNSCOPABLE_ACTIONS.includes(action);
	};

	/** Whether the current selection can be narrowed to specific resources. */
	const isScopable = createMemo(() => isActionScopable(selectedResourceType(), selectedAction()));

	/**
	 * Grant or revoke an action that applies to every resource of its type. These
	 * have no scope to choose, so they are toggled directly rather than being
	 * configured in the resources column.
	 */
	const toggleUnscopableAction = (resourceType: string, action: string) => {
		const permissionId = permissionIdFor(resourceType, action);
		if (!permissionId) return;

		const next = { ...props.permissionsData };
		if (next[permissionId]) delete next[permissionId];
		else next[permissionId] = { permissionType: "exclude", resources: [] };
		props.onChange(next);
	};

	// "Specific resources" with nothing ticked stores no permission at all, so it
	// cannot be derived back from the stored value. Remember the picked radio for
	// the permission being edited so the selection doesn't flicker away.
	const [scopeOverride, setScopeOverride] = createSignal<{ permissionId: string; mode: ScopeMode } | undefined>();

	const scopeMode = createMemo<ScopeMode | undefined>(() => {
		const permissionId = currentPermissionId();
		if (!permissionId) return undefined;

		const override = scopeOverride();
		if (override?.permissionId === permissionId) return override.mode;

		const entry = currentEntry();
		if (!entry) return undefined;
		if (entry.permissionType === "include") return "include";
		return entry.resources.length === 0 ? "all" : "exclude";
	});

	const writeEntry = (mode: ScopeMode, resources: string[]) => {
		const permissionId = currentPermissionId();
		if (!permissionId) return;

		const next = { ...props.permissionsData };
		if (mode === "all") {
			next[permissionId] = { permissionType: "exclude", resources: [] };
		} else if (mode === "include") {
			// An `include` with no resources grants nothing, so it is stored as no
			// permission rather than an empty one.
			if (resources.length === 0) delete next[permissionId];
			else next[permissionId] = { permissionType: "include", resources };
		} else {
			next[permissionId] = { permissionType: "exclude", resources };
		}
		props.onChange(next);
	};

	const selectScope = (mode: ScopeMode) => {
		const permissionId = currentPermissionId();
		if (!permissionId) return;
		setScopeOverride({ permissionId, mode });
		// Switching between "only these" and "all except these" keeps the picked
		// resources; switching to "all resources" drops them.
		writeEntry(mode, mode === "all" ? [] : (currentEntry()?.resources ?? []));
	};

	const toggleResource = (resourceId: string) => {
		const mode = scopeMode();
		if (mode !== "include" && mode !== "exclude") return;

		const current = currentEntry()?.resources ?? [];
		writeEntry(
			mode,
			current.includes(resourceId) ? current.filter((id) => id !== resourceId) : [...current, resourceId]
		);
	};

	const toggleWorkspaceLevelType = (resourceType: string) => {
		const permissionId = workspaceLevelPermissionId(resourceType);
		if (!permissionId) return;

		const next = { ...props.permissionsData };
		if (next[permissionId]) delete next[permissionId];
		else next[permissionId] = { permissionType: "exclude", resources: [] };
		props.onChange(next);
	};

	const isWorkspaceLevelSelected = createMemo(() => workspaceLevelResourceTypes.has(selectedResourceType()));

	const resourcesQuery = useWorkspaceResourcesQuery(
		() => props.workspaceId,
		() => (isScopable() ? selectedResourceType() : undefined)
	);
	const allResources = () => resourcesQuery.data?.pages.flatMap((page) => page.items) ?? [];

	const humanResourceType = createMemo(() => {
		const resourceType = selectedResourceType();
		if (resourceType === "containerRegistryRepository") return "Container Registry Repositories";
		return `${parseCamelCase(resourceType)}s`;
	});

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
									  Workspace-level types carry no actions and no resources, so
									  there is nothing to drill into — a checkbox reads as "switch
									  this on" where the pills read as "drill into this".

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
										onChange={() => {
											setSelectedResourceType(card.resourceType);
											setSelectedAction("");
											toggleWorkspaceLevelType(card.resourceType);
										}}
									/>
								}
							>
								<Pill
									label={parseCamelCase(card.resourceType)}
									badge={resourceTypeBadge(card.resourceType)}
									selected={selectedResourceType() === card.resourceType}
									onClick={() => {
										setSelectedResourceType(card.resourceType);
										setSelectedAction("");
									}}
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
									<Show
										when={isActionScopable(selectedResourceType(), action)}
										fallback={
											/*
											  Nothing to scope, so the action is just on or off — a
											  checkbox, like the workspace-level types in column 1.
											*/
											<Checkbox
												size="sm"
												class={CHECKBOX_CARD_CLASS}
												label={parseCamelCase(action)}
												checked={() => isActionGranted(selectedResourceType(), action)}
												onChange={() => {
													setSelectedAction(action);
													toggleUnscopableAction(selectedResourceType(), action);
												}}
											/>
										}
									>
										<Pill
											label={parseCamelCase(action)}
											badge={actionBadge(action)}
											selected={selectedAction() === action}
											onClick={() => setSelectedAction(action)}
										/>
									</Show>
								)}
							</For>
						</Show>
					</Show>
				</Column>

				<Column heading="Resources">
					<Show when={selectedResourceType()} fallback={<ColumnHint>Select a resource type</ColumnHint>}>
						<Show
							when={!isWorkspaceLevelSelected()}
							fallback={<ColumnHint>Applies to the whole workspace</ColumnHint>}
						>
							<Show when={selectedAction()} fallback={<ColumnHint>Select an action</ColumnHint>}>
								{/*
								  `create` and `add` bring a resource into existence, so there is
								  nothing yet to scope them to. Those are granted by the checkbox in
								  the actions column, so this column only explains itself.
								*/}
								<Show
									when={isScopable()}
									fallback={
										<ColumnHint>This action cannot be limited to specific resources</ColumnHint>
									}
								>
									{/*
									  Pinned above the list so the scope switch and the heading stay
									  visible while a long list of resources scrolls under them. The
									  negative margins let it span the column's padding.
									*/}
									<div class="sticky top-0 z-10 -mx-2 -mt-2 px-2 pt-2 pb-2 bg-secondary-light flex flex-col gap-2">
										<div class="flex gap-1 rounded-xs bg-secondary p-1">
											<ScopeTab
												label="All"
												title={`All ${humanResourceType().toLowerCase()}`}
												selected={scopeMode() === "all"}
												onClick={() => selectScope("all")}
											/>
											<ScopeTab
												label="Specific"
												title={`Only specific ${humanResourceType().toLowerCase()}`}
												selected={scopeMode() === "include"}
												onClick={() => selectScope("include")}
											/>
											<ScopeTab
												label="Except"
												title={`All ${humanResourceType().toLowerCase()} except`}
												selected={scopeMode() === "exclude"}
												onClick={() => selectScope("exclude")}
											/>
										</div>

										<div class="border-t border-border-color/40" />

										<Show when={scopeMode() === "include" || scopeMode() === "exclude"}>
											<ListHeading>
												{scopeMode() === "include" ? "Included" : "Excluded"}{" "}
												{humanResourceType().toLowerCase()}
											</ListHeading>
										</Show>
									</div>

									<Show
										when={scopeMode() === "include" || scopeMode() === "exclude"}
										fallback={<ColumnHint>Pick a scope to choose resources</ColumnHint>}
									>
										<Show
											when={!resourcesQuery.isPending}
											fallback={
												<div class="flex items-center gap-2 text-grey text-xs">
													<LoadingSpinner size={14} />
													<span>Loading {humanResourceType().toLowerCase()}...</span>
												</div>
											}
										>
											<Show
												when={allResources().length > 0}
												fallback={
													<ColumnHint>
														No {humanResourceType().toLowerCase()} found
													</ColumnHint>
												}
											>
												<For each={allResources()}>
													{(resource) => (
														<Checkbox
															size="sm"
															class={CHECKBOX_ROW_CLASS}
															label={resource.name}
															checked={() =>
																(currentEntry()?.resources ?? []).includes(resource.id)
															}
															onChange={() => toggleResource(resource.id)}
														/>
													)}
												</For>
												<Show when={resourcesQuery.hasNextPage}>
													<Button
														variant={ButtonVariant.Plain}
														onClick={() => resourcesQuery.fetchNextPage()}
														disabled={resourcesQuery.isFetchingNextPage}
														class="text-xs cursor-pointer"
													>
														{resourcesQuery.isFetchingNextPage ? "Loading..." : "Load more"}
													</Button>
												</Show>
											</Show>
										</Show>
									</Show>
								</Show>
							</Show>
						</Show>
					</Show>
				</Column>
			</div>
		</Suspense>
	);
};

export default PermissionMatrix;
