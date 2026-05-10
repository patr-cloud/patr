import { FiTrash2 } from "solid-icons/fi";
import { createEffect, createMemo, createSignal, createUniqueId, Index, Show } from "solid-js";
import { Button, ButtonVariant, Input, InputType, InputLabel } from "~/components";
import { Color } from "~/utils/color";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface EnvInputProps {
	/** Current environment variables (source of truth). */
	value: MaybeAccessor<Record<string, string>>;
	/** Fires whenever the committed (validated) map changes. */
	onChange: (next: Record<string, string>) => void;
	/** Fires whenever the validity of the rows changes. Parents use this to gate submit. */
	onValidityChange?: (valid: boolean) => void;
	/** Disables all inputs. */
	disabled?: MaybeAccessor<boolean>;
	/** Additional class for the root container. */
	class?: MaybeAccessor<string>;
}

type Row = { id: string; key: string; value: string };

const makeDraftRow = (): Row => ({ id: createUniqueId(), key: "", value: "" });

const EnvInput = (props: EnvInputProps) => {
	const [rows, setRows] = createSignal<Row[]>([makeDraftRow()]);

	let lastSeeded: Record<string, string> | null = null;
	createEffect(() => {
		const incoming = get(props.value) ?? {};
		if (lastSeeded !== null) {
			const incomingKeys = Object.keys(incoming);
			const same =
				incomingKeys.length === Object.keys(lastSeeded).length &&
				incomingKeys.every((k) => incoming[k] === lastSeeded![k]);
			if (same) return;
		}
		lastSeeded = { ...incoming };
		const seeded: Row[] = Object.entries(incoming).map(([key, value]) => ({
			id: createUniqueId(),
			key,
			value,
		}));
		seeded.push(makeDraftRow());
		setRows(seeded);
	});

	const keyCounts = createMemo(() => {
		const counts = new Map<string, number>();
		for (const row of rows()) {
			if (row.key === "") continue;
			counts.set(row.key, (counts.get(row.key) ?? 0) + 1);
		}
		return counts;
	});

	type RowErrors = { key?: string; value?: string };
	const rowError = (row: Row): RowErrors => {
		const errs: RowErrors = {};
		const keyEmpty = row.key === "";
		const isEmpty = row.value === "";
		if (keyEmpty && isEmpty) return errs;
		if (keyEmpty) errs.key = "Key required";
		if (isEmpty) errs.value = "Value required";
		if (!keyEmpty && (keyCounts().get(row.key) ?? 0) > 1) errs.key = "Duplicate key";
		return errs;
	};

	const hasAnyError = createMemo(() => rows().some((r) => Object.keys(rowError(r)).length > 0));

	const committedMap = (): Record<string, string> => {
		const out: Record<string, string> = {};
		const counts = keyCounts();
		for (const row of rows()) {
			if (row.key === "" || row.value === "") continue;
			if ((counts.get(row.key) ?? 0) > 1) continue;
			out[row.key] = row.value;
		}
		return out;
	};

	createEffect(() => {
		props.onChange(committedMap());
		props.onValidityChange?.(!hasAnyError());
	});

	const updateRow = (id: string, patch: Partial<Pick<Row, "key" | "value">>) => {
		setRows((prev) => {
			const next = prev.map((r) => (r.id === id ? { ...r, ...patch } : r));
			const last = next[next.length - 1];
			if (!last || last.key !== "" || last.value !== "") {
				next.push(makeDraftRow());
			}
			return next;
		});
	};

	const removeRow = (id: string) => {
		setRows((prev) => {
			const next = prev.filter((r) => r.id !== id);
			const last = next[next.length - 1];
			if (!last || last.key !== "" || last.value !== "") {
				next.push(makeDraftRow());
			}
			return next;
		});
	};

	const handleBlur = (id: string) => {
		setRows((prev) => {
			const row = prev.find((r) => r.id === id);
			if (!row) return prev;
			if (row.key !== "" || row.value !== "") return prev;
			if (prev.length === 1) return prev;
			if (prev[prev.length - 1].id === id) return prev;
			return prev.filter((r) => r.id !== id);
		});
	};

	return (
		<div class={`flex gap-8 items-start w-full ${get(props.class) ?? ""}`}>
			<InputLabel parentClass="flex-2 pt-2.5" label="Environment Variables" />

			<div class="flex flex-col flex-10 gap-4 w-full">
				<Index each={rows()}>
					{(row) => {
						const errs = () => rowError(row());
						const keyErr = () => errs().key;
						const valueErr = () => errs().value;
						const isDraftTrailing = () =>
							row().key === "" && row().value === "" && rows()[rows().length - 1]?.id === row().id;

						return (
							<div class="flex flex-col gap-1 w-full">
								<div class="flex items-center flex-10 gap-4">
									<Input
										class={`flex-5 ${keyErr() ? "border-error!" : ""}`}
										disabled={get(props.disabled)}
										placeholder="Enter Env Name"
										type={InputType.Text}
										value={row().key}
										onInput={(e) => updateRow(row().id, { key: e.currentTarget.value })}
										onBlur={() => handleBlur(row().id)}
										onKeyDown={(e) => {
											if (e.key === "Enter") e.preventDefault();
										}}
									/>
									<Input
										class={`flex-7 ${valueErr() ? "border-error!" : ""}`}
										disabled={get(props.disabled)}
										placeholder="Enter Env Value"
										type={InputType.Text}
										value={row().value}
										onInput={(e) => updateRow(row().id, { value: e.currentTarget.value })}
										onBlur={() => handleBlur(row().id)}
										onKeyDown={(e) => {
											if (e.key === "Enter") e.preventDefault();
										}}
									/>

									<Show
										when={!get(props.disabled) && !isDraftTrailing()}
										fallback={
											<Button
												type="button"
												variant={ButtonVariant.Outlined}
												class="flex-1 h-full flex items-center gap-2 invisible"
												color={Color.Error}
											>
												<FiTrash2 size={16} />
											</Button>
										}
									>
										<Button
											type="button"
											onClick={() => removeRow(row().id)}
											variant={ButtonVariant.Outlined}
											class="flex-1 h-full flex items-center gap-2"
											color={Color.Error}
										>
											<FiTrash2 size={16} />
										</Button>
									</Show>
								</div>

								<Show when={keyErr() || valueErr()}>
									<div class="flex gap-4 pl-0 text-error text-sm">
										<Show when={keyErr()}>
											<span class="flex-5">{keyErr()}</span>
										</Show>
										<Show when={!keyErr()}>
											<span class="flex-5" />
										</Show>
										<Show when={valueErr()}>
											<span class="flex-7">{valueErr()}</span>
										</Show>
										<span class="flex-1" />
									</div>
								</Show>
							</div>
						);
					}}
				</Index>
			</div>
		</div>
	);
};

export default EnvInput;
