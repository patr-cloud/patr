import { FiArrowRight, FiCheck, FiCopy, FiTrash2 } from "solid-icons/fi";
import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import { ExposedPortType } from "~/bindings";
import { Input, InputType, Label } from "~/components";
import { DEPLOYMENT_DOMAIN } from "~/utils/env";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface PortInputProps {
	/**
	 * Saved ports — pass the server's current state here, not the in-progress
	 * edit state. The component seeds its rows from this map and layers
	 * user edits on top internally; `onChange` emits the merged result.
	 * A row's public URL is only shown while its (port, type) still exactly
	 * matches this map. On the create page, pass an empty map.
	 */
	value: MaybeAccessor<Record<string, ExposedPortType | undefined>>;
	/** Fires whenever the committed (validated) map of current rows changes. */
	onChange: (next: Record<string, ExposedPortType>) => void;
	/** Fires whenever the validity of the rows changes. */
	onValidityChange?: (valid: boolean) => void;
	/** If set, HTTP rows pointing at a saved port show their public URL. */
	deploymentId?: string;
	/** Disables all inputs. */
	disabled?: MaybeAccessor<boolean>;
	/** Additional class for the root container. */
	class?: MaybeAccessor<string>;
}

type Row = { port: string; type: ExposedPortType };

const DEFAULT_TYPE: ExposedPortType = "http";

/** A single exposed port: the port, its public URL (once saved), and its actions. */
const PortRow = (props: {
	row: Row;
	/** The public URL without the scheme, once the port is saved. */
	url?: string;
	/** Shown in place of the URL while the port has none yet. */
	pendingText?: string;
	disabled: boolean;
	onRemove: () => void;
}) => {
	const [copied, setCopied] = createSignal(false);

	const copy = async () => {
		if (!props.url) return;
		try {
			await navigator.clipboard.writeText(`https://${props.url}`);
			setCopied(true);
			setTimeout(() => setCopied(false), 2000);
		} catch (error) {
			console.error("Failed to copy:", error);
		}
	};

	return (
		<div class="group flex items-center gap-4 py-3 min-w-0">
			<span class="w-14 shrink-0 font-log text-white">{props.row.port}</span>
			<Show when={props.url || props.pendingText}>
				<FiArrowRight size={14} class="shrink-0 text-gray-500" />
			</Show>
			<div class="flex-1 min-w-0">
				<Show
					when={props.url}
					fallback={
						<Show when={props.pendingText}>
							<span class="text-sm italic text-gray-500 truncate block">{props.pendingText}</span>
						</Show>
					}
				>
					{(url) => (
						<a
							href={`https://${url()}`}
							target="_blank"
							rel="noopener noreferrer"
							class="block w-fit max-w-full truncate font-log text-sm text-gray-400 transition-colors group-hover:text-primary group-focus-within:text-primary"
						>
							{url()}
						</a>
					)}
				</Show>
			</div>
			<div class="flex items-center gap-2 shrink-0">
				<Show when={props.url}>
					<button
						type="button"
						onClick={copy}
						title={copied() ? "Copied!" : "Copy URL"}
						aria-label="Copy URL"
						class="p-1.5 rounded-xs cursor-pointer text-gray-500 transition-colors group-hover:text-gray-300 hover:text-white! hover:bg-white/5"
					>
						{copied() ? <FiCheck size={16} /> : <FiCopy size={16} />}
					</button>
				</Show>
				<Show when={!props.disabled}>
					<button
						type="button"
						onClick={() => props.onRemove()}
						title="Remove port"
						aria-label={`Remove port ${props.row.port}`}
						class="p-1.5 rounded-xs cursor-pointer text-gray-500 transition-colors group-hover:text-gray-300 hover:text-error! hover:bg-white/5"
					>
						<FiTrash2 size={16} />
					</button>
				</Show>
			</div>
		</div>
	);
};

const PortInput = (props: PortInputProps) => {
	const [rows, setRows] = createSignal<Row[]>([]);
	const [draft, setDraft] = createSignal("");

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
		setRows(
			Object.entries(incoming).map(([port, type]) => ({
				port,
				type: (type ?? DEFAULT_TYPE) as ExposedPortType,
			}))
		);
	});

	const validatePort = (value: string): string | undefined => {
		const n = Number(value);
		if (!Number.isInteger(n) || String(n) !== value) return "Must be a number";
		if (n < 1 || n > 65535) return "Port out of range";
		if (rows().some((r) => r.port === value)) return "Duplicate port";
		return undefined;
	};

	// Leftover text that isn't a valid port keeps the section invalid, so it
	// can't be silently dropped on save.
	const draftError = createMemo(() => {
		const value = draft().trim();
		return value === "" ? undefined : validatePort(value);
	});

	createEffect(() => {
		const out: Record<string, ExposedPortType> = {};
		for (const row of rows()) {
			out[row.port] = row.type;
		}
		props.onChange(out);
		props.onValidityChange?.(!draftError());
	});

	const addDraft = () => {
		const value = draft().trim();
		if (value === "" || validatePort(value) !== undefined) return;
		setRows((prev) => [...prev, { port: value, type: DEFAULT_TYPE }]);
		setDraft("");
	};

	const removeRow = (port: string) => setRows((prev) => prev.filter((r) => r.port !== port));

	const urlFor = (row: Row) => {
		const saved = get(props.value) ?? {};
		if (!DEPLOYMENT_DOMAIN || !props.deploymentId || row.type !== "http" || saved[row.port] !== "http") {
			return undefined;
		}
		return `${row.port}-${props.deploymentId}.${DEPLOYMENT_DOMAIN}`;
	};

	const pendingTextFor = (row: Row) => {
		if (!DEPLOYMENT_DOMAIN || row.type !== "http") return undefined;
		return props.deploymentId ? "URL available after update" : "URL assigned once created";
	};

	return (
		<div class={`flex gap-8 items-start w-full ${get(props.class) ?? ""}`}>
			<Label
				parentClass="flex-2 pt-2.5"
				label="Exposed Ports"
				comments="Ports your container listens on. Each one gets a public HTTPS URL once the deployment is saved."
			/>

			<div class="flex flex-col flex-10 w-full min-w-0">
				{/* Dividers sit between rows only, so the last row has no line under it. */}
				<div class="flex flex-col divide-y divide-border-color">
					<For each={rows()}>
						{(row) => (
							<PortRow
								row={row}
								url={urlFor(row)}
								pendingText={pendingTextFor(row)}
								disabled={!!get(props.disabled)}
								onRemove={() => removeRow(row.port)}
							/>
						)}
					</For>
				</div>

				<Show
					when={!get(props.disabled)}
					fallback={
						<Show when={rows().length === 0}>
							<span class="py-3 text-sm text-gray-500">No ports exposed</span>
						</Show>
					}
				>
					<Input
						class={`${draftError() ? "border-error!" : ""}`}
						placeholder="Add a port"
						type={InputType.Text}
						endIcon={() => (
							<kbd class="font-log text-xs text-gray-400 border border-border-color rounded-xs px-2 py-0.5">
								Enter
							</kbd>
						)}
						value={draft()}
						onInput={(e) => setDraft(e.currentTarget.value)}
						onBlur={addDraft}
						onKeyDown={(e) => {
							if (e.key === "Enter") {
								e.preventDefault();
								addDraft();
							} else if (e.key === "Escape") {
								setDraft("");
							}
						}}
					/>
					<Show when={draftError()}>
						<div class="text-error text-sm mt-1">{draftError()}</div>
					</Show>
				</Show>
			</div>
		</div>
	);
};

export default PortInput;
