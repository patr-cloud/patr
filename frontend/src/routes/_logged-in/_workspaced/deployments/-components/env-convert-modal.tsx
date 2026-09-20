import { FiEye, FiEyeOff } from "solid-icons/fi";
import { Accessor, createMemo, createSignal, For, Setter, Show } from "solid-js";
import { CreateSecretRequest, CreateSecretResponse, EnvironmentVariableValue } from "~/bindings";
import { Alert, Button, ButtonVariant, Checkbox, Modal, ModalContainer } from "~/components";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

interface EnvConvertModalProps {
	isOpen: Accessor<boolean>;
	setIsOpen: Setter<boolean>;
	/** Plain-string env vars on the deployment, in display order. */
	convertible: Accessor<Array<{ key: string; value: string }>>;
	/** Names already taken by a secret in this workspace, lowercased. */
	existingSecretNames: Accessor<Set<string>>;
	/** Fires with the keys that became secrets, mapped to their references. */
	onConverted: (converted: Record<string, EnvironmentVariableValue>) => void;
}

const EnvConvertModal = (props: EnvConvertModalProps) => {
	const [workspaceId] = useLastWorkspaceId();

	const [selected, setSelected] = createSignal<Set<string>>(new Set());
	const [revealed, setRevealed] = createSignal<Set<string>>(new Set());
	const [submitting, setSubmitting] = createSignal(false);
	const [failures, setFailures] = createSignal<Array<{ key: string; error: string }>>([]);

	// A key whose name is already a secret here would fail on the workspace's
	// unique index, so it can't be selected.
	const isTaken = (key: string) => props.existingSecretNames().has(key.toLowerCase());

	const selectable = createMemo(() => props.convertible().filter((entry) => !isTaken(entry.key)));

	const toggle = (key: string, checked: boolean) => {
		setSelected((prev) => {
			const next = new Set(prev);
			if (checked) {
				next.add(key);
			} else {
				next.delete(key);
			}
			return next;
		});
	};

	const toggleReveal = (key: string) => {
		setRevealed((prev) => {
			const next = new Set(prev);
			if (next.has(key)) {
				next.delete(key);
			} else {
				next.add(key);
			}
			return next;
		});
	};

	const allSelected = () => selectable().length > 0 && selected().size === selectable().length;

	const toggleAll = (checked: boolean) =>
		setSelected(checked ? new Set<string>(selectable().map((entry) => entry.key)) : new Set<string>());

	const canSubmit = () => !submitting() && selected().size > 0;

	const reset = () => {
		setSelected(new Set<string>());
		setRevealed(new Set<string>());
		setFailures([]);
		setSubmitting(false);
	};

	const handleClose = () => {
		props.setIsOpen(false);
		reset();
	};

	// One request per secret, with no transaction across them: apply whatever
	// succeeded and leave the rest selected so the user can retry.
	const handleSubmit = async () => {
		const wsId = workspaceId();
		if (!wsId) return;

		setSubmitting(true);
		setFailures([]);

		const converted: Record<string, EnvironmentVariableValue> = {};
		const failed: Array<{ key: string; error: string }> = [];

		for (const entry of props.convertible()) {
			if (!selected().has(entry.key)) continue;

			const body: CreateSecretRequest = { name: entry.key, value: entry.value };
			const response = await httpRequest<CreateSecretResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/secret`,
				{ method: "POST", body: JSON.stringify(body) }
			);

			if (response.ok) {
				converted[entry.key] = { fromSecret: response.data.id };
			} else {
				failed.push({ key: entry.key, error: response.data.error });
			}
		}

		if (Object.keys(converted).length > 0) {
			props.onConverted(converted);
			setSelected((prev) => {
				const next = new Set(prev);
				for (const key of Object.keys(converted)) next.delete(key);
				return next;
			});
		}

		setSubmitting(false);
		setFailures(failed);

		if (failed.length === 0) {
			handleClose();
		}
	};

	return (
		<Modal
			isOpen={props.isOpen}
			setIsOpen={props.setIsOpen}
			renderTrigger={() => <></>}
			renderModalContent={() => (
				<ModalContainer closeFn={handleClose} width="min(640px, 100%)" class="max-h-[80vh] overflow-y-auto">
					<h2 class="text-lg text-primary font-semibold mb-1">Convert to secrets</h2>
					<p class="text-sm text-white mb-4">
						The values you pick are stored as workspace secrets, and this deployment keeps a
						reference to them instead of the value. Save the deployment to apply.
					</p>

					<Show
						when={props.convertible().length > 0}
						fallback={
							<p class="text-sm text-white/70">
								No environment variables to convert &mdash; they're either empty or already
								secrets.
							</p>
						}
					>
						<div class="flex items-center justify-between mb-3 text-sm">
							<Checkbox
								checked={allSelected}
								disabled={() => selectable().length === 0}
								onChange={toggleAll}
								label="Select all"
							/>
							<span class="text-white/70">
								{selected().size} of {selectable().length} selected
							</span>
						</div>

						<div class="flex flex-col gap-2 max-h-[50vh] overflow-y-auto pr-1">
							<For each={props.convertible()}>
								{(entry) => {
									const taken = () => isTaken(entry.key);
									const isRevealed = () => revealed().has(entry.key);

									return (
										<div class="flex flex-col gap-1">
											<div class="flex items-center gap-3">
												<Checkbox
													checked={() => selected().has(entry.key)}
													disabled={() => taken() || submitting()}
													onChange={(checked) => toggle(entry.key, checked)}
												/>
												<span class="flex-5 font-mono text-sm text-white truncate">
													{entry.key}
												</span>
												<span class="flex-6 font-mono text-sm text-white/70 truncate">
													{isRevealed() ? entry.value : "••••••••"}
												</span>
												<button
													type="button"
													class="text-white/70 hover:text-white cursor-pointer"
													aria-label={isRevealed() ? "Hide value" : "Show value"}
													onClick={() => toggleReveal(entry.key)}
												>
													{isRevealed() ? <FiEyeOff size={16} /> : <FiEye size={16} />}
												</button>
											</div>

											<Show when={taken()}>
												<Alert
													type="warning"
													message={`A secret named "${entry.key}" already exists in this workspace`}
												/>
											</Show>
										</div>
									);
								}}
							</For>
						</div>
					</Show>

					<Show when={failures().length > 0}>
						<div class="flex flex-col gap-1 mt-4">
							<For each={failures()}>
								{(failure) => (
									<Alert
										type="error"
										message={`${failure.key}: ${failure.error}`}
									/>
								)}
							</For>
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
							{submitting() ? "Converting…" : "Convert to secrets"}
						</Button>
					</div>
				</ModalContainer>
			)}
		/>
	);
};

export default EnvConvertModal;
