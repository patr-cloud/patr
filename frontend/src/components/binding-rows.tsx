import { createMemo, Index, JSX, Show } from "solid-js";
import { FiPlus, FiX } from "solid-icons/fi";
import InputDropdown, { InputDropdownOption } from "./input-dropdown";
import ScopePicker from "./scope-picker";
import { Scope } from "~/utils/scope";

/**
 * One binding, as the editor holds it mid-edit. [subjectId] is empty on a row
 * the user has just added and not filled in yet — the parent drops those before
 * saving.
 */
export interface Binding {
	subjectId: string;
	scope: Scope;
}

interface BindingRowsProps {
	workspaceId: string;
	bindings: Binding[];
	/** Called with the complete next list on every edit. */
	onChange: (next: Binding[]) => void;
	/**
	 * What the row's first column picks. Members and invites pick the role, with
	 * the actor fixed; a role's users tab picks the user, with the role fixed.
	 */
	subjectOptions: InputDropdownOption[];
	subjectPlaceholder: string;
	/**
	 * The role whose permissions bound the row's resource types, given the row's
	 * subject. Identity on the member screens (the subject *is* the role); the
	 * fixed role on a role's users tab.
	 */
	scopeRoleId: (subjectId: string) => string;
	addLabel?: string;
	emptyText?: string;
	/** Rendered under the add button — e.g. a link off to create a new role. */
	footer?: () => JSX.Element;
}

/**
 * Edits a list of bindings as one row each: what is granted, and where it
 * applies (see [ScopePicker]). A new row starts unset and workspace-wide — the
 * same reach the pre-scoped world gave — and narrowing it is opt-in.
 *
 * A subject already used by another row is not offered again: two rows for the
 * same role would be two grants the backend would merge anyway, and the scope
 * picker already takes a set of resources.
 */
const BindingRows = (props: BindingRowsProps) => {
	const subjectLabels = createMemo(() => new Map(props.subjectOptions.map((o) => [o.value, o.label])));

	const optionsFor = (index: number) => {
		const takenElsewhere = new Set(
			props.bindings.filter((_, i) => i !== index).map((binding) => binding.subjectId)
		);
		return props.subjectOptions.filter((option) => !takenElsewhere.has(option.value));
	};

	const updateAt = (index: number, next: Binding) =>
		props.onChange(props.bindings.map((binding, i) => (i === index ? next : binding)));

	const removeAt = (index: number) => props.onChange(props.bindings.filter((_, i) => i !== index));

	const addRow = () => props.onChange([...props.bindings, { subjectId: "", scope: { scopeType: "workspace" } }]);

	// One unfilled row at a time — adding another before naming this one just
	// stacks empty rows the parent would drop on save.
	const hasUnfilledRow = createMemo(() => props.bindings.some((binding) => !binding.subjectId));

	const canAddMore = createMemo(() => props.bindings.length < props.subjectOptions.length);

	return (
		<div class="flex flex-col gap-3">
			<Show
				when={props.bindings.length > 0}
				fallback={<p class="text-grey text-sm italic">{props.emptyText ?? "Nothing assigned."}</p>}
			>
				{/* `Index`, not `For`: rows are positional and their contents change on
				  every edit. `For` keys by object identity, so handing it a fresh
				  binding object per keystroke tore each row down and rebuilt it —
				  which slammed the resource dropdown shut after a single tick and
				  made multi-select impossible. */}
				<ul class="flex flex-col gap-2">
					<Index each={props.bindings}>
						{(binding, index) => (
							<li class="flex flex-col gap-2 p-3 border border-border-color rounded-xs">
								{/* One row: what is granted, where it applies, and — when that is
								  a set of resources — which type and which of them. The wide
								  detail panel is what makes this fit; the chips wrap underneath. */}
								<div class="flex items-start gap-2">
									<div class="w-full max-w-56 shrink-0">
										<InputDropdown
											placeholder={props.subjectPlaceholder}
											options={optionsFor(index)}
											value={() => binding().subjectId || undefined}
											onSelect={(subjectId) => updateAt(index, { ...binding(), subjectId })}
										/>
									</div>
									<div class="flex-1 min-w-0">
										{/* Scope needs a subject first: without one there are no
										  permissions to derive resource types from. */}
										<Show
											when={binding().subjectId}
											fallback={
												<div class="h-9 flex items-center text-grey text-xs">
													Pick one to set where it applies.
												</div>
											}
										>
											<ScopePicker
												orientation="inline"
												workspaceId={props.workspaceId}
												roleId={props.scopeRoleId(binding().subjectId)}
												scope={binding().scope}
												onChange={(scope) => updateAt(index, { ...binding(), scope })}
											/>
										</Show>
									</div>
									<button
										type="button"
										aria-label={`Remove ${subjectLabels().get(binding().subjectId) ?? "binding"}`}
										onClick={() => removeAt(index)}
										class="shrink-0 text-grey hover:text-error transition-colors cursor-pointer p-2"
									>
										<FiX size={16} />
									</button>
								</div>
							</li>
						)}
					</Index>
				</ul>
			</Show>

			<Show when={canAddMore()}>
				<button
					type="button"
					onClick={addRow}
					disabled={hasUnfilledRow()}
					class="self-start flex items-center gap-2 text-primary text-sm hover:underline cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed disabled:no-underline"
				>
					<FiPlus size={14} />
					{props.addLabel ?? "Add binding"}
				</button>
			</Show>

			<Show when={props.footer}>{(footer) => footer()()}</Show>
		</div>
	);
};

export default BindingRows;
