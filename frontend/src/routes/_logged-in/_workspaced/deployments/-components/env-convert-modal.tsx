import { Accessor, createEffect, createSignal, For, Setter, Show, untrack } from "solid-js";
import { CreateSecretRequest, CreateSecretResponse, EnvironmentVariableValue } from "~/bindings";
import { useQueryClient } from "@tanstack/solid-query";
import { Alert, Button, ButtonVariant, Checkbox, Input, InputType, Modal, ModalContainer } from "~/components";
import { secretKeys } from "~/hooks/query-keys";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

interface EnvConvertModalProps {
	isOpen: Accessor<boolean>;
	setIsOpen: Setter<boolean>;
	/** Plain-string env vars on the deployment, in display order. */
	convertible: Accessor<Array<{ key: string; value: string }>>;
	/** Names already taken by a secret in this workspace, lowercased. */
	existingSecretNames: Accessor<Set<string>>;
	/** Keys to tick when the modal opens. Empty opens with nothing selected. */
	initialSelection?: Accessor<string[]>;
	/** Fires with the keys that became secrets, mapped to their references. */
	onConverted: (converted: Record<string, EnvironmentVariableValue>) => void;
}

const EnvConvertModal = (props: EnvConvertModalProps) => {
	const [workspaceId] = useLastWorkspaceId();
	const queryClient = useQueryClient();

	const [selected, setSelected] = createSignal<Set<string>>(new Set());
	const [submitting, setSubmitting] = createSignal(false);
	const [failures, setFailures] = createSignal<Array<{ key: string; error: string }>>([]);

	// Per-row edits, keyed by the env var's key. The name starts blank — a
	// secret's name is a workspace-wide label, not the deployment's variable
	// name, so it is the user's to choose. The value is seeded from the row.
	const [names, setNames] = createSignal<Record<string, string>>({});
	const [values, setValues] = createSignal<Record<string, string>>({});

	const nameOf = (key: string) => names()[key] ?? "";
	const valueOf = (key: string) => values()[key] ?? "";

	// Opening from a per-value hint ticks that one key; opening from the button
	// ticks nothing. Seeded on open rather than on mount, since the modal stays
	// mounted between openings.
	createEffect(() => {
		if (!props.isOpen()) return;
		untrack(() => {
			setSelected(new Set(props.initialSelection?.() ?? []));
			setNames({});
			setValues(Object.fromEntries(props.convertible().map((entry) => [entry.key, entry.value])));
		});
	});

	// A name already used by a secret here would fail on the workspace's unique
	// index. The name is editable, so this is a live warning rather than a
	// block — renaming clears it.
	const isTaken = (key: string) => props.existingSecretNames().has(nameOf(key).trim().toLowerCase());

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

	const allSelected = () => props.convertible().length > 0 && selected().size === props.convertible().length;

	const toggleAll = (checked: boolean) =>
		setSelected(checked ? new Set<string>(props.convertible().map((entry) => entry.key)) : new Set<string>());

	const canSubmit = () => !submitting() && selected().size > 0;

	/** Selected rows still missing a name. The server would reject those. */
	const unnamed = () => [...selected()].filter((key) => nameOf(key).trim() === "");

	const reset = () => {
		setSelected(new Set<string>());
		setNames({});
		setValues({});
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

		// Nothing is sent until every selected row is named — the inline errors
		// beside those rows are already showing why.
		if (unnamed().length > 0) return;

		setSubmitting(true);
		setFailures([]);

		const converted: Record<string, EnvironmentVariableValue> = {};
		const failed: Array<{ key: string; error: string }> = [];

		for (const entry of props.convertible()) {
			if (!selected().has(entry.key)) continue;

			const body: CreateSecretRequest = { name: nameOf(entry.key).trim(), value: valueOf(entry.key) };
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
			// The new secrets have to show up in the row dropdowns and in the
			// taken-name check, both of which read the workspace's secret list.
			queryClient.invalidateQueries({ queryKey: secretKeys.all(wsId) });
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
						The values you pick are stored as workspace secrets, and this deployment keeps a reference to
						them instead of the value. Save the deployment to apply.
					</p>

					<Show
						when={props.convertible().length > 0}
						fallback={
							<p class="text-sm text-white/70">
								No environment variables to convert &mdash; they're either empty or already secrets.
							</p>
						}
					>
						<div class="flex items-center justify-between mb-3 text-sm">
							<Checkbox
								checked={allSelected}
								disabled={() => props.convertible().length === 0}
								onChange={toggleAll}
								label="Select all"
							/>
							<span class="text-white/70">
								{selected().size} of {props.convertible().length} selected
							</span>
						</div>

						<div class="flex flex-col gap-3 max-h-[50vh] overflow-y-auto pr-1">
							<For each={props.convertible()}>
								{(entry) => {
									const taken = () => isTaken(entry.key);
									// Only nags about rows that are actually going to be sent.
									const needsName = () =>
										selected().has(entry.key) && nameOf(entry.key).trim() === "";

									return (
										<div class="flex flex-col gap-1">
											<div class="flex items-center gap-3">
												<Checkbox
													checked={() => selected().has(entry.key)}
													disabled={submitting}
													onChange={(checked) => toggle(entry.key, checked)}
												/>
												<Input
													class={`flex-5 ${needsName() ? "border-error!" : ""}`}
													disabled={submitting()}
													placeholder="Secret name"
													type={InputType.Text}
													value={nameOf(entry.key)}
													onInput={(e) =>
														setNames((prev) => ({
															...prev,
															[entry.key]: e.currentTarget.value,
														}))
													}
												/>
												<Input
													class="flex-6"
													disabled={submitting()}
													placeholder="Secret value"
													type={InputType.Text}
													value={valueOf(entry.key)}
													onInput={(e) =>
														setValues((prev) => ({
															...prev,
															[entry.key]: e.currentTarget.value,
														}))
													}
												/>
											</div>

											<Show when={needsName()}>
												<Alert type="error" message="Give this secret a name" />
											</Show>

											<Show when={taken()}>
												<Alert
													type="warning"
													message={`A secret named "${nameOf(entry.key).trim()}" already exists in this workspace`}
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
								{(failure) => <Alert type="error" message={`${failure.key}: ${failure.error}`} />}
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
