import { FiPlus, FiTrash } from "solid-icons/fi";
import { createEffect, createMemo, createSignal, createUniqueId, Index, Show } from "solid-js";
import { Base64String } from "~/bindings";
import { Button, ButtonVariant, FileInput, Input, InputLabel, InputType } from "~/components";
import { Color } from "~/utils/color";
import { convertFileToBase64, get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

const MAX_SIZE_BYTES = 1 * 1024 * 1024; // 1 MB

interface ConfigMountProps {
	/** Current config mounts: path → base64-encoded file contents. */
	value: MaybeAccessor<Record<string, Base64String>>;
	/** Fires when the committed map changes. */
	onChange: (next: Record<string, Base64String>) => void;
	/** Disables all inputs. */
	disabled?: MaybeAccessor<boolean>;
}

type Row = { id: string; path: string; content: Base64String; fileLabel: string };

const ConfigMount = (props: ConfigMountProps) => {
	const [rows, setRows] = createSignal<Row[]>([]);
	const [draftPath, setDraftPath] = createSignal<string>("");
	const [draftContent, setDraftContent] = createSignal<Base64String | null>(null);
	const [draftFileLabel, setDraftFileLabel] = createSignal<string>("");
	const [error, setError] = createSignal<string | null>(null);

	const committedMap = (): Record<string, Base64String> => {
		const out: Record<string, Base64String> = {};
		const counts = new Map<string, number>();
		for (const row of rows()) {
			if (row.path === "") continue;
			counts.set(row.path, (counts.get(row.path) ?? 0) + 1);
		}
		for (const row of rows()) {
			if (row.path === "") continue;
			if ((counts.get(row.path) ?? 0) > 1) continue;
			out[row.path] = row.content;
		}
		return out;
	};

	// Seed from parent on mount / external change.
	createEffect(() => {
		const incoming = get(props.value) ?? {};
		const currentCommitted = committedMap();
		const sameKeys =
			Object.keys(incoming).length === Object.keys(currentCommitted).length &&
			Object.keys(incoming).every((k) => incoming[k] === currentCommitted[k]);
		if (sameKeys) return;
		setRows(
			Object.entries(incoming).map(([path, content]) => ({
				id: createUniqueId(),
				path,
				content,
				fileLabel: "",
			}))
		);
	});

	// Emit committed map whenever rows change.
	createEffect(() => {
		props.onChange(committedMap());
	});

	const pathCounts = createMemo(() => {
		const counts = new Map<string, number>();
		for (const row of rows()) {
			if (row.path === "") continue;
			counts.set(row.path, (counts.get(row.path) ?? 0) + 1);
		}
		return counts;
	});

	const rowError = (row: Row): string | undefined => {
		if (row.path === "") return "Path required";
		if ((pathCounts().get(row.path) ?? 0) > 1) return "Duplicate path";
		return undefined;
	};

	const handleDraftFileChange = async (e: Event & { currentTarget: HTMLInputElement }) => {
		const file = e.currentTarget.files?.[0];
		if (!file) return;
		if (file.size > MAX_SIZE_BYTES) {
			setError("File size exceeds the maximum limit of 1 MB.");
			return;
		}
		setError(null);
		const b64 = await convertFileToBase64(file);
		setDraftContent(b64);
		setDraftFileLabel(file.name);
	};

	const addDraft = () => {
		const path = draftPath().trim();
		const content = draftContent();
		if (!path || !content) {
			setError("Please provide both a file path and a file.");
			return;
		}
		if (pathCounts().get(path) ?? 0) {
			setError(`A mount with path "${path}" already exists.`);
			return;
		}
		setRows((prev) => [...prev, { id: createUniqueId(), path, content, fileLabel: draftFileLabel() }]);
		setDraftPath("");
		setDraftContent(null);
		setDraftFileLabel("");
		setError(null);
	};

	const updateRowPath = (id: string, path: string) => {
		setRows((prev) => prev.map((r) => (r.id === id ? { ...r, path } : r)));
	};

	const replaceRowFile = async (id: string, e: Event & { currentTarget: HTMLInputElement }) => {
		const file = e.currentTarget.files?.[0];
		if (!file) return;
		if (file.size > MAX_SIZE_BYTES) {
			setError("File size exceeds the maximum limit of 1 MB.");
			return;
		}
		setError(null);
		const b64 = await convertFileToBase64(file);
		setRows((prev) => prev.map((r) => (r.id === id ? { ...r, content: b64, fileLabel: file.name } : r)));
	};

	const removeRow = (id: string) => {
		setRows((prev) => prev.filter((r) => r.id !== id));
	};

	return (
		<div class="flex flex-col gap-0 w-full">
			<Show when={!get(props.disabled)}>
				<div class="flex gap-8 items-center w-full">
					<InputLabel parentClass="flex-2" label="Config File" />
					<section class="flex-10 flex items-center gap-4 w-full">
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
							class="flex-5"
							id="deployment-config-filename"
							name="deployment-config-filename"
							placeholder="Mount path (e.g. /etc/my.conf)"
						/>
						<FileInput
							id="deployment-config"
							name="deployment-config"
							onChange={handleDraftFileChange}
							class="flex-7"
							placeholder={draftFileLabel() || "Select Config File"}
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
				</div>

				<Show when={error()}>
					<p class="text-sm text-error mt-1 ml-20">{error()}</p>
				</Show>
			</Show>

			<Show when={get(props.disabled) && rows().length > 0}>
				<div class="flex gap-8 items-center w-full">
					<InputLabel parentClass="flex-2" label="Config File" />
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
										class={`flex-5 ${err() ? "border-error!" : ""}`}
										disabled={get(props.disabled)}
										type={InputType.Text}
										name="deployment-config-filename"
										placeholder="Mount path"
										value={row().path}
										onInput={(e) => updateRowPath(row().id, e.currentTarget.value)}
									/>
									<Show
										when={!get(props.disabled)}
										fallback={
											<Input
												type={InputType.Text}
												value={row().fileLabel || "File uploaded"}
												class="flex-7"
												disabled
											/>
										}
									>
										<FileInput
											name="deployment-config"
											onChange={(e) => replaceRowFile(row().id, e)}
											class="flex-7"
											placeholder={row().fileLabel || "Replace file"}
										/>
									</Show>

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
		</div>
	);
};

export default ConfigMount;
