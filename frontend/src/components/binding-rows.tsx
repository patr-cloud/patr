import { Accessor, createEffect, createMemo, createSignal, For, JSX, Show, untrack } from "solid-js";
import Button from "./button";
import Input from "./input";
import InputDropdown, { InputDropdownOption } from "./input-dropdown";
import ScopePicker from "./scope-picker";
import BindingRow, { BINDING_GRID } from "./binding-row";
import { useResourcesInfoQuery } from "~/hooks/fetch";
import { ButtonVariant } from "~/utils/color";
import { Scope } from "~/utils/scope";

/** One binding: who or what is granted, and where it applies. */
export interface Binding {
	subjectId: string;
	scope: Scope;
}

/** A stable string form of a binding list, so two lists compare equal regardless of order. */
const canonicalBindings = (bindings: Binding[]) =>
	JSON.stringify(
		bindings
			.map((binding) => [
				binding.subjectId,
				binding.scope.scopeType === "workspace" ? "workspace" : [...binding.scope.resources].sort(),
			])
			.sort(([a], [b]) => String(a).localeCompare(String(b)))
	);

/**
 * A draft of a saved binding list, for pages that save it with one button.
 *
 * The draft follows `saved` while it is clean, so refetches land, and is left
 * alone while dirty, so a refetch never wipes an edit in progress.
 */
export const createBindingsDraft = (saved: Accessor<Binding[]>) => {
	const [baseline, setBaseline] = createSignal<Binding[]>([]);
	const [draft, setDraft] = createSignal<Binding[]>([]);

	const isDirty = () => canonicalBindings(draft()) !== canonicalBindings(baseline());

	createEffect(() => {
		const next = saved();
		if (untrack(isDirty)) return;
		setBaseline(next);
		setDraft(next);
	});

	/** Throws the draft away, back to what is saved now. */
	const discard = () => {
		const next = untrack(saved);
		setBaseline(next);
		setDraft(next);
	};

	/** Marks the draft as saved, so it is clean until the next edit. */
	const markSaved = () => setBaseline(draft());

	return { draft, setDraft, isDirty, discard, markSaved };
};

interface BindingRowsProps {
	workspaceId: string;
	bindings: Binding[];
	/** Called with the complete next list on every edit. */
	onChange: (next: Binding[]) => void;
	/**
	 * What a binding grants. Members and invites pick the role, with the actor
	 * fixed; a role's users tab picks the user, with the role fixed.
	 */
	subjectOptions: InputDropdownOption[];
	subjectPlaceholder: string;
	/**
	 * The role whose permissions bound a binding's resource types, given its
	 * subject. Identity on the member screens (the subject *is* the role); the
	 * fixed role on a role's users tab.
	 */
	scopeRoleId: (subjectId: string) => string;
	addLabel?: string;
	emptyText?: string;
	/** Hides the add row and the row actions. */
	readOnly?: boolean;
	/** Rendered under the box — e.g. a link off to create a new role. */
	footer?: () => JSX.Element;
}

/**
 * Edits a list of bindings like the API token editor: an add row on top, and a
 * table of bindings below, editable one row at a time. Every change goes
 * through `onChange`; saving is the page's job.
 *
 * A subject already bound is not offered again: two bindings for the same role
 * would be two grants the backend merges anyway, and a scope already takes a
 * set of resources.
 */
const BindingRows = (props: BindingRowsProps) => {
	const [editingIndex, setEditingIndex] = createSignal<number>();
	const [newSubjectId, setNewSubjectId] = createSignal<string>();
	const [newScope, setNewScope] = createSignal<Scope>({ scopeType: "workspace" });

	const subjectLabels = createMemo(() => new Map(props.subjectOptions.map((o) => [o.value, o.label])));
	const subjectLabel = (subjectId: string) => subjectLabels().get(subjectId) ?? subjectId;

	/** The subjects not bound by any row other than `exceptIndex`. */
	const freeSubjects = (exceptIndex?: number) => {
		const taken = new Set(props.bindings.filter((_, i) => i !== exceptIndex).map((binding) => binding.subjectId));
		return props.subjectOptions.filter((option) => !taken.has(option.value));
	};

	// Resolve scoped resource ids to names for the table, in one lookup.
	const resourceIds = createMemo(() => [
		...new Set(
			props.bindings.flatMap((binding) =>
				binding.scope.scopeType === "resources" ? binding.scope.resources : []
			)
		),
	]);
	const resourcesInfoQuery = useResourcesInfoQuery(
		() => resourceIds(),
		() => props.workspaceId
	);
	const resourceName = (resourceId: string) => {
		// Checked through status first so a cold lookup doesn't suspend the table.
		if (!resourcesInfoQuery.isSuccess || resourcesInfoQuery.isPlaceholderData) return "…";
		return resourcesInfoQuery.data?.get(resourceId)?.name || "Unknown resource";
	};

	// Another subject can reach other resource types, so its scope starts over.
	const selectNewSubject = (subjectId: string) => {
		setNewSubjectId(subjectId);
		setNewScope({ scopeType: "workspace" });
	};

	const canAdd = () => {
		const scope = newScope();
		return !!newSubjectId() && (scope.scopeType === "workspace" || scope.resources.length > 0);
	};

	const add = () => {
		const subjectId = newSubjectId();
		if (!subjectId) return;
		props.onChange([...props.bindings, { subjectId, scope: newScope() }]);
		setNewSubjectId();
		setNewScope({ scopeType: "workspace" });
	};

	const replaceAt = (index: number, next: Binding) => {
		props.onChange(props.bindings.map((binding, i) => (i === index ? next : binding)));
		setEditingIndex();
	};

	const removeAt = (index: number) => {
		props.onChange(props.bindings.filter((_, i) => i !== index));
		setEditingIndex();
	};

	return (
		<div class="flex flex-col gap-2 w-full">
			<div class="flex flex-col gap-3 w-full border border-border-color rounded-xs p-4">
				<Show when={!props.readOnly}>
					<div class={`${BINDING_GRID} items-start p-sm border border-border-color rounded-xs`}>
						<InputDropdown
							styleVariant="medium"
							placeholder={props.subjectPlaceholder}
							options={freeSubjects()}
							value={() => newSubjectId()}
							onSelect={selectNewSubject}
						/>
						<div class="sm:col-span-2 min-w-0">
							{/* Scope needs a subject first: without one there are no
							  permissions to derive resource types from. */}
							<Show
								when={newSubjectId()}
								fallback={<Input styleVariant="medium" value="Entire workspace" disabled />}
							>
								{(subjectId) => (
									<ScopePicker
										orientation="inline"
										styleVariant="medium"
										workspaceId={props.workspaceId}
										roleId={props.scopeRoleId(subjectId())}
										scope={newScope()}
										onChange={setNewScope}
									/>
								)}
							</Show>
						</div>
						<Button type="button" variant={ButtonVariant.Contained} disabled={!canAdd()} onClick={add}>
							{props.addLabel ?? "Add"}
						</Button>
					</div>
				</Show>

				<Show
					when={props.bindings.length > 0}
					fallback={<p class="text-grey text-sm italic">{props.emptyText ?? "Nothing assigned."}</p>}
				>
					<ul class="flex flex-col">
						<For each={props.bindings}>
							{(binding, index) => (
								<BindingRow
									workspaceId={props.workspaceId}
									binding={binding}
									subjectLabel={subjectLabel}
									subjectOptions={freeSubjects(index())}
									subjectPlaceholder={props.subjectPlaceholder}
									scopeRoleId={props.scopeRoleId}
									resourceName={resourceName}
									editing={editingIndex() === index()}
									readOnly={!!props.readOnly}
									onEdit={() => setEditingIndex(index())}
									onDone={(next) => replaceAt(index(), next)}
									onCancel={() => setEditingIndex()}
									onRemove={() => removeAt(index())}
								/>
							)}
						</For>
					</ul>
				</Show>
			</div>

			<Show when={!props.readOnly && props.footer}>{(footer) => footer()()}</Show>
		</div>
	);
};

export default BindingRows;
