import { FiTrash2, FiUploadCloud } from "solid-icons/fi";
import { Accessor, createMemo, createSignal, createUniqueId, Index, Setter, Show } from "solid-js";
import { Alert, Button, ButtonVariant, Input, InputType, Modal, ModalContainer } from "~/components";
import { Color } from "~/utils/color";

interface EnvUploadModalProps {
	isOpen: Accessor<boolean>;
	setIsOpen: Setter<boolean>;
	/** Keys already on the deployment, mapped to whether the existing value is secret-backed. */
	existingKeys: Accessor<Map<string, boolean>>;
	onSubmit: (entries: Array<{ key: string; value: string }>) => void;
}

type Row = { id: string; key: string; value: string };

const KEY_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;

const makeDraft = (): Row => ({ id: createUniqueId(), key: "", value: "" });

const unescapeDoubleQuoted = (s: string): string =>
	s.replace(/\\(.)/g, (_, c) => {
		switch (c) {
			case "n":
				return "\n";
			case "r":
				return "\r";
			case "t":
				return "\t";
			case "\\":
				return "\\";
			case '"':
				return '"';
			default:
				return c;
		}
	});

const stripInlineComment = (s: string): string => {
	// Drop the first " #" (or "\t#") and everything after it.
	const m = s.match(/\s+#.*$/);
	return m ? s.slice(0, m.index).trimEnd() : s.trimEnd();
};

export const parseDotEnv = (text: string): Array<{ key: string; value: string }> => {
	const out = new Map<string, string>();
	for (const rawLine of text.split(/\r?\n/)) {
		const line = rawLine.trim();
		if (line === "" || line.startsWith("#")) continue;
		const stripped = line.startsWith("export ") ? line.slice("export ".length).trimStart() : line;
		const eq = stripped.indexOf("=");
		if (eq < 0) continue;
		const key = stripped.slice(0, eq).trim();
		if (!KEY_RE.test(key)) continue;
		let value = stripped.slice(eq + 1).trimStart();
		if (value.length >= 2) {
			const first = value[0];
			const last = value[value.length - 1];
			if (first === '"' && last === '"') {
				value = unescapeDoubleQuoted(value.slice(1, -1));
			} else if (first === "'" && last === "'") {
				value = value.slice(1, -1);
			} else {
				value = stripInlineComment(value);
			}
		} else {
			value = stripInlineComment(value);
		}
		out.set(key, value);
	}
	return Array.from(out, ([key, value]) => ({ key, value }));
};

const EnvUploadModal = (props: EnvUploadModalProps) => {
	const [fileName, setFileName] = createSignal<string | null>(null);
	const [rows, setRows] = createSignal<Row[]>([]);
	const [isDragging, setIsDragging] = createSignal(false);
	let fileInputRef: HTMLInputElement | undefined;

	const reset = () => {
		setFileName(null);
		setRows([]);
		if (fileInputRef) fileInputRef.value = "";
	};

	const ingestFile = async (file: File) => {
		const text = await file.text();
		const parsed = parseDotEnv(text);
		const seeded: Row[] = parsed.map((p) => ({ id: createUniqueId(), ...p }));
		seeded.push(makeDraft());
		setFileName(file.name);
		setRows(seeded);
	};

	const onFileChange = async (e: Event & { currentTarget: HTMLInputElement }) => {
		const file = e.currentTarget.files?.[0];
		if (!file) return;
		await ingestFile(file);
	};

	const onDrop = async (e: DragEvent) => {
		e.preventDefault();
		setIsDragging(false);
		const file = e.dataTransfer?.files?.[0];
		if (!file) return;
		await ingestFile(file);
	};

	const updateRow = (id: string, patch: Partial<Pick<Row, "key" | "value">>) => {
		setRows((prev) => {
			const next = prev.map((r) => (r.id === id ? { ...r, ...patch } : r));
			const last = next[next.length - 1];
			if (!last || last.key !== "" || last.value !== "") next.push(makeDraft());
			return next;
		});
	};

	const removeRow = (id: string) => {
		setRows((prev) => {
			const next = prev.filter((r) => r.id !== id);
			const last = next[next.length - 1];
			if (!last || last.key !== "" || last.value !== "") next.push(makeDraft());
			return next;
		});
	};

	const keyCounts = createMemo(() => {
		const m = new Map<string, number>();
		for (const r of rows()) {
			if (r.key === "") continue;
			m.set(r.key, (m.get(r.key) ?? 0) + 1);
		}
		return m;
	});

	type RowErrs = { key?: string; value?: string };
	const rowError = (row: Row): RowErrs => {
		const errs: RowErrs = {};
		const keyEmpty = row.key === "";
		const valEmpty = row.value === "";
		if (keyEmpty && valEmpty) return errs;
		if (keyEmpty) errs.key = "Key required";
		else if (!KEY_RE.test(row.key)) errs.key = "Invalid key";
		else if ((keyCounts().get(row.key) ?? 0) > 1) errs.key = "Duplicate key";
		else if (props.existingKeys().get(row.key) === true)
			errs.key = "Bound to a secret — remove this row to keep it";
		if (valEmpty) errs.value = "Value required";
		return errs;
	};

	// Collides with an existing plain value: overwriting is the point of uploading
	// a .env, so this informs rather than blocks.
	const rowWarning = (row: Row): string | undefined => {
		if (rowError(row).key) return undefined;
		if (row.key === "") return undefined;
		return props.existingKeys().get(row.key) === false ? "Will update existing value" : undefined;
	};

	const hasError = createMemo(() => rows().some((r) => Object.keys(rowError(r)).length > 0));
	const nonEmptyCount = createMemo(() => rows().filter((r) => r.key !== "" || r.value !== "").length);
	const canSubmit = () => fileName() !== null && !hasError() && nonEmptyCount() > 0;

	const handleSubmit = () => {
		const entries: Array<{ key: string; value: string }> = [];
		for (const r of rows()) {
			if (r.key === "" || r.value === "") continue;
			entries.push({ key: r.key, value: r.value });
		}
		props.onSubmit(entries);
		props.setIsOpen(false);
		reset();
	};

	const handleClose = () => {
		props.setIsOpen(false);
		reset();
	};

	return (
		<Modal
			isOpen={props.isOpen}
			setIsOpen={props.setIsOpen}
			renderTrigger={() => <></>}
			renderModalContent={() => (
				<ModalContainer closeFn={handleClose} width="min(640px, 100%)" class="max-h-[80vh] overflow-y-auto">
					<h2 class="text-lg text-primary font-semibold mb-1">Upload .env file</h2>
					<p class="text-sm text-white mb-4">We'll parse it and let you review before adding.</p>

					<Show
						when={fileName() !== null}
						fallback={
							<>
								<input
									ref={fileInputRef}
									type="file"
									accept=".env,text/plain"
									class="hidden"
									onChange={onFileChange}
								/>
								<div
									role="button"
									tabindex="0"
									onClick={() => fileInputRef?.click()}
									onKeyDown={(e) => {
										if (e.key === "Enter" || e.key === " ") {
											e.preventDefault();
											fileInputRef?.click();
										}
									}}
									onDragOver={(e) => {
										e.preventDefault();
										setIsDragging(true);
									}}
									onDragLeave={() => setIsDragging(false)}
									onDrop={onDrop}
									class={`flex flex-col items-center justify-center gap-3 px-6 py-10 border-2 border-dashed rounded-md cursor-pointer transition-colors ${isDragging()
											? "border-primary bg-primary/8"
											: "border-primary/30 hover:border-primary/60 hover:bg-primary/4"
										}`}
								>
									<FiUploadCloud size={40} class="text-white" />
									<div class="text-center">
										<p class="text-base font-medium text-white">Drag a .env file here</p>
										<p class="text-sm text-white/70 mt-1">
											or <span class="underline">click to browse</span>
										</p>
									</div>
								</div>
							</>
						}
					>
						<div class="flex items-center justify-between mb-3 text-sm">
							<span class="text-white">
								Parsed <strong>{nonEmptyCount()}</strong>{" "}
								{nonEmptyCount() === 1 ? "variable" : "variables"} from{" "}
								<span class="font-mono">{fileName()}</span>
							</span>
							<button type="button" class="text-primary hover:underline" onClick={reset}>
								Choose a different file
							</button>
						</div>

						<div class="flex flex-col gap-3 max-h-[50vh] overflow-y-auto pr-1">
							<Index each={rows()}>
								{(row) => {
									const errs = () => rowError(row());
									const warn = () => rowWarning(row());
									const isDraftTrailing = () =>
										row().key === "" &&
										row().value === "" &&
										rows()[rows().length - 1]?.id === row().id;
									return (
										<div class="flex flex-col gap-1">
											<div class="flex flex-col sm:flex-row sm:items-center gap-3">
												<Input
													styleVariant="medium"
													class={`flex-5 ${errs().key ? "border-error!" : ""}`}
													placeholder="KEY"
													type={InputType.Text}
													value={row().key}
													onInput={(e) => updateRow(row().id, { key: e.currentTarget.value })}
												/>
												<Input
													styleVariant="medium"
													class={`flex-7 ${errs().value ? "border-error!" : ""}`}
													placeholder="value"
													type={InputType.Text}
													value={row().value}
													onInput={(e) =>
														updateRow(row().id, { value: e.currentTarget.value })
													}
												/>
												<Show
													when={!isDraftTrailing()}
													fallback={<div class="hidden sm:block w-9" />}
												>
													<Button
														type="button"
														onClick={() => removeRow(row().id)}
														variant={ButtonVariant.Outlined}
														color={Color.Error}
														class="h-full flex items-center justify-center"
													>
														<FiTrash2 size={16} />
													</Button>
												</Show>
											</div>
											<Show when={errs().key || errs().value || warn()}>
												<div class="flex flex-col gap-0.5">
													<Show when={errs().key}>
														{(msg) => <Alert type="error" message={msg()} />}
													</Show>
													<Show when={errs().value}>
														{(msg) => <Alert type="error" message={msg()} />}
													</Show>
													<Show when={warn()}>
														{(msg) => <Alert type="warning" message={msg()} />}
													</Show>
												</div>
											</Show>
										</div>
									);
								}}
							</Index>
						</div>
					</Show>

					<div class="flex justify-between gap-3 mt-6">
						<Button
							type="button"
							variant={ButtonVariant.Plain}
							onClick={handleClose}
							class="cursor-pointer"
						>
							Cancel
						</Button>
						<Button
							type="button"
							variant={ButtonVariant.Contained}
							disabled={!canSubmit()}
							onClick={handleSubmit}
							class="cursor-pointer"
						>
							Add to deployment
						</Button>
					</div>
				</ModalContainer>
			)}
		/>
	);
};

export default EnvUploadModal;
