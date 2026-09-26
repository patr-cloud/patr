import { FiPlus, FiTrash } from "solid-icons/fi";
import { createEffect, createMemo, createSignal, createUniqueId, Index, Show } from "solid-js";
import { VolumeConfig } from "~/bindings";
import { Button, ButtonVariant, Input, Label, InputType } from "~/components";
import { Color } from "~/utils/color";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

const MAX_PATH_LENGTH = 4096;

interface VolumeMountProps {
	/** Current volumes: mount path → per-volume config (empty today). */
	value: MaybeAccessor<Record<string, VolumeConfig>>;
	/** Fires when the committed map changes. */
	onChange: (next: Record<string, VolumeConfig>) => void;
	/** Fires whenever the validity of the rows changes. Parents gate submit on this. */
	onValidityChange?: (valid: boolean) => void;
	/** Config mount paths on the same deployment — a volume can't share a path with one. */
	configMountPaths?: MaybeAccessor<string[]>;
	/** Disables all inputs. */
	disabled?: MaybeAccessor<boolean>;
}

type Row = { id: string; path: string };

/**
 * Mirrors the server's `deployment_volume` path CHECK: absolute, `/`-separated
 * non-empty segments, no trailing slash, no `.` or `..` segments.
 */
const pathError = (path: string): string | undefined => {
	if (path === "") return "Path required";
	if (!path.startsWith("/")) return "Must be an absolute path";
	if (path === "/") return "Cannot be the filesystem root";
	if (path.endsWith("/")) return "Must not end with a slash";
	if (path.length > MAX_PATH_LENGTH) return "Path is too long";
	for (const segment of path.split("/").slice(1)) {
		if (segment === "") return "Must not contain empty path segments";
		if (segment === "." || segment === "..") return "Must not contain . or .. segments";
	}
	return undefined;
};

const VolumeMount = (props: VolumeMountProps) => {
	const [rows, setRows] = createSignal<Row[]>([]);
	const [draftPath, setDraftPath] = createSignal<string>("");
	const [error, setError] = createSignal<string | null>(null);

	const committedMap = (): Record<string, VolumeConfig> => {
		const out: Record<string, VolumeConfig> = {};
		for (const row of rows()) {
			if (rowError(row) !== undefined) continue;
			out[row.path] = {};
		}
		return out;
	};

	// Seed from props.value on mount and whenever the incoming map *itself*
	// changes (e.g. the parent refetches after a save). This effect must not
	// read our own committed state — doing so would make it re-run on every
	// keystroke and clobber the user's edits.
	let lastSeeded: string[] | null = null;
	createEffect(() => {
		const incoming = Object.keys(get(props.value) ?? {});
		if (lastSeeded !== null) {
			const same = incoming.length === lastSeeded.length && incoming.every((path, i) => path === lastSeeded![i]);
			if (same) return;
		}
		lastSeeded = [...incoming];
		setRows(incoming.map((path) => ({ id: createUniqueId(), path })));
	});

	const pathCounts = createMemo(() => {
		const counts = new Map<string, number>();
		for (const row of rows()) {
			if (row.path === "") continue;
			counts.set(row.path, (counts.get(row.path) ?? 0) + 1);
		}
		return counts;
	});

	const configMountPaths = createMemo(() => new Set(get(props.configMountPaths) ?? []));

	const rowError = (row: Row): string | undefined => {
		const err = pathError(row.path);
		if (err) return err;
		if ((pathCounts().get(row.path) ?? 0) > 1) return "Duplicate path";
		if (configMountPaths().has(row.path)) return "A config file is already mounted at this path";
		return undefined;
	};

	const hasAnyError = createMemo(() => rows().some((r) => rowError(r) !== undefined));

	// Emit committed map + validity whenever rows change.
	createEffect(() => {
		props.onChange(committedMap());
		props.onValidityChange?.(!hasAnyError());
	});

	const addDraft = () => {
		const path = draftPath().trim();
		const err = pathError(path);
		if (err) {
			setError(err);
			return;
		}
		if (pathCounts().get(path) ?? 0) {
			setError(`A volume at "${path}" already exists.`);
			return;
		}
		setRows((prev) => [...prev, { id: createUniqueId(), path }]);
		setDraftPath("");
		setError(null);
	};

	const updateRowPath = (id: string, path: string) => {
		setRows((prev) => prev.map((r) => (r.id === id ? { ...r, path } : r)));
	};

	const removeRow = (id: string) => {
		setRows((prev) => prev.filter((r) => r.id !== id));
	};

	return (
		<div class="flex flex-col gap-0 w-full">
			<Show when={!get(props.disabled)}>
				{/* Top-aligned, with the error inside the input column: the label
				    is taller than the input, so hanging the error off the row would
				    leave it closer to the next row than to the input it belongs to. */}
				<div class="flex gap-8 items-start w-full">
					<Label
						parentClass="flex-2"
						label="Volumes"
						comments="Directories that persist across restarts. Single replica only."
					/>
					<div class="flex-10 flex flex-col gap-1 w-full">
						<section class="flex items-center gap-4 w-full">
							<Input
								type={InputType.Text}
								value={draftPath()}
								onInput={(e) => setDraftPath(e.currentTarget.value)}
								onKeyDown={(e) => {
									if (e.key === "Enter") {
										e.preventDefault();
										addDraft();
									}
								}}
								class="flex-12"
								id="deployment-volume-path"
								name="deployment-volume-path"
								placeholder="Mount path (e.g. /data)"
							/>

							<Button
								type="button"
								variant={ButtonVariant.Contained}
								class="flex-1"
								onClick={(e) => {
									e.preventDefault();
									addDraft();
								}}
							>
								<FiPlus size={16} />
							</Button>
						</section>

						<Show when={error()}>
							<p class="text-sm text-error">{error()}</p>
						</Show>
					</div>
				</div>
			</Show>

			<Show when={get(props.disabled) && rows().length > 0}>
				<div class="flex gap-8 items-center w-full">
					<Label parentClass="flex-2" label="Volumes" />
					<div class="flex-10" />
				</div>
			</Show>

			<Index each={rows()}>
				{(row) => {
					const err = () => rowError(row());
					return (
						<div class="flex flex-col gap-1 w-full mt-3">
							<div class="flex gap-8 items-center w-full">
								<div class="flex-2" />
								<section class="flex-10 flex items-center gap-4 w-full">
									<Input
										class={`flex-12 ${err() ? "border-error!" : ""}`}
										disabled={get(props.disabled)}
										type={InputType.Text}
										name="deployment-volume-path"
										placeholder="Mount path"
										value={row().path}
										onInput={(e) => updateRowPath(row().id, e.currentTarget.value)}
									/>

									<Show when={!get(props.disabled)} fallback={<div class="flex-1" />}>
										<Button
											type="button"
											variant={ButtonVariant.Contained}
											color={Color.Error}
											class="flex-1"
											onClick={(e) => {
												e.preventDefault();
												removeRow(row().id);
											}}
										>
											<FiTrash size={16} />
										</Button>
									</Show>
								</section>
							</div>
							<Show when={err()}>
								<div class="flex gap-8 w-full">
									<div class="flex-2" />
									<div class="flex-10 text-error text-sm">{err()}</div>
								</div>
							</Show>
						</div>
					);
				}}
			</Index>

			<Show when={!get(props.disabled) && rows().length > 0}>
				<div class="flex gap-8 w-full mt-2">
					<div class="flex-2" />
					<small class="flex-10 text-xxs text-grey">
						Removing a volume keeps its data on the runner; re-adding the same path restores it.
					</small>
				</div>
			</Show>
		</div>
	);
};

export default VolumeMount;
