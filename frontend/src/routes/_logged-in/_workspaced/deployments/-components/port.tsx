import { FiExternalLink, FiTrash2 } from "solid-icons/fi";
import { createEffect, createMemo, createSignal, createUniqueId, Index, Show } from "solid-js";
import { ExposedPortType } from "~/bindings";
import { Button, ButtonVariant, Input, InputDropdown, InputLabel, InputType } from "~/components";
import { Color } from "~/utils/color";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface PortInputProps {
	/**
	 * Saved ports — pass the server's current state here, not the in-progress
	 * edit state. The component seeds its rows from this map and layers
	 * user edits on top internally; `onChange` emits the merged result.
	 * The Visit URL link is only rendered for rows whose (port, type) still
	 * exactly matches this map. On the create page, pass an empty map.
	 */
	value: MaybeAccessor<Record<string, ExposedPortType | undefined>>;
	/** Fires whenever the committed (validated) map of current rows changes. */
	onChange: (next: Record<string, ExposedPortType>) => void;
	/** Fires whenever the validity of the rows changes. */
	onValidityChange?: (valid: boolean) => void;
	/** If set, HTTP rows pointing at a saved port show a "Visit URL" link. */
	deploymentId?: string;
	/** Disables all inputs. */
	disabled?: MaybeAccessor<boolean>;
	/** Additional class for the root container. */
	class?: MaybeAccessor<string>;
}

type Row = { id: string; port: string; type: ExposedPortType };

const DEFAULT_TYPE: ExposedPortType = "http";

const makeDraftRow = (): Row => ({ id: createUniqueId(), port: "", type: DEFAULT_TYPE });

const TYPE_OPTIONS = [
	{ value: "http", label: "HTTP" },
	{ value: "tcp", label: "TCP" },
	{ value: "udp", label: "UDP" },
];

const PortInput = (props: PortInputProps) => {
	const [rows, setRows] = createSignal<Row[]>([makeDraftRow()]);

	// Seed from props.value on mount and whenever the incoming map *itself*
	// changes (e.g. the parent refetches after a save). Crucially this effect
	// must not re-read our own committed state — doing so would make it
	// re-run on every keystroke and clobber the user's edits.
	let lastSeeded: Record<string, ExposedPortType | undefined> | null = null;
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
		const seeded = Object.entries(incoming).map(([port, type]) => ({
			id: createUniqueId(),
			port,
			type: (type ?? DEFAULT_TYPE) as ExposedPortType,
		}));
		seeded.push(makeDraftRow());
		setRows(seeded);
	});

	const portCounts = createMemo(() => {
		const counts = new Map<string, number>();
		for (const row of rows()) {
			if (row.port === "") continue;
			counts.set(row.port, (counts.get(row.port) ?? 0) + 1);
		}
		return counts;
	});

	const rowError = (row: Row): string | undefined => {
		if (row.port === "") return undefined;
		const n = Number(row.port);
		if (!Number.isInteger(n) || String(n) !== row.port) return "Must be a number";
		if (n < 1 || n > 65535) return "Port out of range";
		if ((portCounts().get(row.port) ?? 0) > 1) return "Duplicate port";
		return undefined;
	};

	const hasAnyError = createMemo(() => rows().some((r) => rowError(r) !== undefined));

	const committedMap = (): Record<string, ExposedPortType> => {
		const out: Record<string, ExposedPortType> = {};
		const counts = portCounts();
		for (const row of rows()) {
			if (rowError(row) !== undefined) continue;
			if (row.port === "") continue;
			if ((counts.get(row.port) ?? 0) > 1) continue;
			out[row.port] = row.type;
		}
		return out;
	};

	createEffect(() => {
		props.onChange(committedMap());
		props.onValidityChange?.(!hasAnyError());
	});

	const updateRow = (id: string, patch: Partial<Pick<Row, "port" | "type">>) => {
		setRows((prev) => {
			const next = prev.map((r) => (r.id === id ? { ...r, ...patch } : r));
			const last = next[next.length - 1];
			if (!last || last.port !== "") {
				next.push(makeDraftRow());
			}
			return next;
		});
	};

	const removeRow = (id: string) => {
		setRows((prev) => {
			const next = prev.filter((r) => r.id !== id);
			if (next.length === 0 || next[next.length - 1].port !== "") {
				next.push(makeDraftRow());
			}
			return next;
		});
	};

	const handleBlur = (id: string) => {
		setRows((prev) => {
			const row = prev.find((r) => r.id === id);
			if (!row) return prev;
			if (row.port !== "") return prev;
			if (prev.length === 1) return prev;
			if (prev[prev.length - 1].id === id) return prev;
			return prev.filter((r) => r.id !== id);
		});
	};

	return (
		<div class={`flex gap-8 items-start w-full ${get(props.class) ?? ""}`}>
			<InputLabel parentClass="flex-2 pt-2.5" label="Exposed Ports" />

			<div class="flex flex-col flex-10 gap-4 w-full">
				<Index each={rows()}>
					{(row) => {
						const err = createMemo(() => rowError(row()));
						const isDraftTrailing = createMemo(
							() => row().port === "" && rows()[rows().length - 1]?.id === row().id
						);
						const showVisitUrl = createMemo(() => {
							const r = row();
							const saved = get(props.value) ?? {};
							return (
								r.port.trim() !== "" &&
								r.type === "http" &&
								!!props.deploymentId &&
								!err() &&
								saved[r.port] === "http"
							);
						});

						return (
							<div class="flex flex-col gap-1 w-full">
								<div class="flex items-center gap-4 w-full">
									<Input
										class={`flex-5 ${err() ? "border-error!" : ""}`}
										disabled={get(props.disabled)}
										placeholder="Enter Port Number"
										type={InputType.Text}
										value={row().port}
										onInput={(e) => updateRow(row().id, { port: e.currentTarget.value })}
										onBlur={() => handleBlur(row().id)}
										onKeyDown={(e) => {
											if (e.key === "Enter") e.preventDefault();
										}}
									/>
									<div class="flex-7 flex items-center gap-4">
										<InputDropdown
											class="flex-3"
											disabled={get(props.disabled)}
											placeholder="Type"
											value={row().type}
											onSelect={(value) =>
												updateRow(row().id, { type: value as ExposedPortType })
											}
											options={TYPE_OPTIONS}
										/>
										<Show when={showVisitUrl()} fallback={<div class="flex-9" />}>
											<a
												class="flex-9 flex items-center justify-center gap-2 rounded-xs bg-secondary-light border border-secondary-medium py-xs text-primary"
												href={`https://${row().port}-${props.deploymentId}.onpatr.cloud`}
												target="_blank"
											>
												<FiExternalLink size={16} />
												Visit URL
											</a>
										</Show>
									</div>
									<Show
										when={!get(props.disabled) && !isDraftTrailing()}
										fallback={
											<Button
												type="button"
												variant={ButtonVariant.Outlined}
												class="flex-1 invisible"
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
											class="flex-1"
											color={Color.Error}
										>
											<FiTrash2 size={16} />
										</Button>
									</Show>
								</div>

								<Show when={err()}>
									<div class="text-error text-sm">{err()}</div>
								</Show>
							</div>
						);
					}}
				</Index>
			</div>
		</div>
	);
};

export default PortInput;
