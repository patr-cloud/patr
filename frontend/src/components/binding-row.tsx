import { createSignal, Show, untrack } from "solid-js";
import { FiX } from "solid-icons/fi";
import InputDropdown, { InputDropdownOption } from "./input-dropdown";
import ScopePicker from "./scope-picker";
import type { Binding } from "./binding-rows";

/** Columns shared by the add row and every table row, so they line up. */
export const BINDING_GRID =
	"grid grid-cols-1 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1.5fr)_auto] gap-2 sm:gap-3";

interface BindingRowProps {
	workspaceId: string;
	binding: Binding;
	subjectLabel: (subjectId: string) => string;
	/** Subjects this row may switch to: every subject not already used by another row. */
	subjectOptions: InputDropdownOption[];
	subjectPlaceholder: string;
	/** Paging and server search for `subjectOptions`; see [BindingRows]. */
	onLoadMoreSubjects?: () => void;
	isLoadingMoreSubjects?: () => boolean;
	onSearchSubjects?: (text: string) => void;
	scopeRoleId: (subjectId: string) => string;
	/** Resolves a scoped resource id to a display name. */
	resourceName: (resourceId: string) => string;
	editing: boolean;
	readOnly: boolean;
	onEdit: () => void;
	/** Commits the edit with the row's new binding. */
	onDone: (next: Binding) => void;
	onCancel: () => void;
	onRemove: () => void;
}

/**
 * One binding in [BindingRows]' table. Read-only until Edit, which swaps the
 * cells for the subject dropdown and a [ScopePicker].
 */
const BindingRow = (props: BindingRowProps) => {
	const label = () => props.subjectLabel(props.binding.subjectId);

	const resourcesText = () =>
		props.binding.scope.scopeType === "workspace"
			? "All resources"
			: props.binding.scope.resources.map(props.resourceName).join(", ") || "None";

	return (
		<li
			class={`${BINDING_GRID} ${props.editing ? "items-start" : "items-center"} py-xs min-h-12 border-b border-border-color last:border-0`}
		>
			<Show
				when={props.editing}
				fallback={
					<>
						<span class="text-white text-sm truncate">{label()}</span>
						<span class="text-grey text-sm truncate">
							{props.binding.scope.scopeType === "workspace" ? "Entire workspace" : "Specific resources"}
						</span>
						<span class="text-grey text-sm truncate" title={resourcesText()}>
							{resourcesText()}
						</span>
						<Show when={!props.readOnly} fallback={<span />}>
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
						</Show>
					</>
				}
			>
				<RowEditor {...props} />
			</Show>
		</li>
	);
};

/** The edit-mode cells of a row. Holds the draft until Done. */
const RowEditor = (props: BindingRowProps) => {
	// The draft is seeded once, when Edit is pressed; later prop changes don't reset it.
	const initial = untrack(() => props.binding);
	const [subjectId, setSubjectId] = createSignal(initial.subjectId);
	const [scope, setScope] = createSignal(initial.scope);

	// Another subject can reach other resource types, so its scope starts over.
	const selectSubject = (id: string) => {
		if (id === subjectId()) return;
		setSubjectId(id);
		setScope({ scopeType: "workspace" });
	};

	// A resource scope with nothing picked would grant nothing.
	const isEmptyResourceScope = () => {
		const current = scope();
		return current.scopeType === "resources" && current.resources.length === 0;
	};

	const done = () => {
		const next = { subjectId: subjectId(), scope: scope() };
		if (JSON.stringify(next) === JSON.stringify(initial)) props.onCancel();
		else props.onDone(next);
	};

	return (
		<>
			<InputDropdown
				styleVariant="medium"
				placeholder={props.subjectPlaceholder}
				options={props.subjectOptions}
				value={subjectId}
				onSelect={selectSubject}
				onLoadMore={props.onLoadMoreSubjects}
				isLoadingMore={props.isLoadingMoreSubjects}
				onSearch={props.onSearchSubjects}
			/>
			<div class="sm:col-span-2 min-w-0">
				<ScopePicker
					orientation="inline"
					styleVariant="medium"
					workspaceId={props.workspaceId}
					roleId={props.scopeRoleId(subjectId())}
					scope={scope()}
					onChange={setScope}
				/>
			</div>
			<div class="flex items-center justify-end gap-3 self-start h-9">
				<button
					type="button"
					onClick={done}
					disabled={isEmptyResourceScope()}
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

export default BindingRow;
