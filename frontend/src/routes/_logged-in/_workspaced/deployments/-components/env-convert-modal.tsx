import { Accessor, createEffect, createSignal, Setter, Show, untrack } from "solid-js";
import { CreateSecretRequest, CreateSecretResponse } from "~/bindings";
import { useQueryClient } from "@tanstack/solid-query";
import { Alert, Button, ButtonVariant, Input, InputType, InputWithLabel, Modal, ModalContainer } from "~/components";
import { secretKeys } from "~/hooks/query-keys";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

interface EnvConvertModalProps {
	isOpen: Accessor<boolean>;
	setIsOpen: Setter<boolean>;
	/** The env var being converted. */
	entry: Accessor<{ key: string; value: string } | null>;
	/** Names already taken by a secret in this workspace, lowercased. */
	existingSecretNames: Accessor<Set<string>>;
	/** Fires with the env var's key and the id of the secret now holding its value. */
	onConverted: (key: string, secretId: string) => void;
}

/** Turns one env var's value into a workspace secret. */
const EnvConvertModal = (props: EnvConvertModalProps) => {
	const [workspaceId] = useLastWorkspaceId();
	const queryClient = useQueryClient();

	// The name starts blank — a secret's name is a workspace-wide label, not the
	// deployment's variable name, so it is the user's to choose. The value is
	// seeded from the row, and can be edited before it's stored.
	const [name, setName] = createSignal("");
	const [value, setValue] = createSignal("");
	const [submitted, setSubmitted] = createSignal(false);
	const [submitting, setSubmitting] = createSignal(false);
	const [error, setError] = createSignal("");

	// Seeded on open rather than on mount, since the modal stays mounted
	// between openings.
	createEffect(() => {
		if (!props.isOpen()) return;
		untrack(() => {
			setName("");
			setValue(props.entry()?.value ?? "");
			setSubmitted(false);
			setError("");
		});
	});

	// Only nags once the user has tried to convert, not the moment it opens.
	const needsName = () => submitted() && name().trim() === "";

	// A name already used by a secret here would fail on the workspace's unique
	// index. It's a live warning rather than a block — renaming clears it.
	const isTaken = () => props.existingSecretNames().has(name().trim().toLowerCase());

	const handleClose = () => props.setIsOpen(false);

	const handleSubmit = async () => {
		const wsId = workspaceId();
		const entry = props.entry();
		if (!wsId || !entry) return;

		setSubmitted(true);
		if (name().trim() === "" || value() === "") return;

		setSubmitting(true);
		setError("");

		const body: CreateSecretRequest = { name: name().trim(), value: value() };
		const response = await httpRequest<CreateSecretResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/secret`,
			{ method: "POST", body: JSON.stringify(body) }
		);

		setSubmitting(false);

		if (!response.ok) {
			setError(
				response.data.error === "resourceAlreadyExists"
					? `A secret named "${name().trim()}" already exists in this workspace`
					: "Failed to create the secret. Please try again."
			);
			return;
		}

		// The new secret has to show up in the row dropdowns and in the
		// taken-name check, both of which read the workspace's secret list.
		queryClient.invalidateQueries({ queryKey: secretKeys.all(wsId) });
		props.onConverted(entry.key, response.data.id);
		handleClose();
	};

	return (
		<Modal
			isOpen={props.isOpen}
			setIsOpen={props.setIsOpen}
			renderTrigger={() => <></>}
			renderModalContent={() => (
				<ModalContainer closeFn={handleClose} width="min(40rem, 100%)" class="text-white">
					<h2 class="text-lg text-primary font-semibold mb-1">Convert to a secret</h2>
					<p class="text-sm text-white mb-6">
						The value of <span class="font-mono">{props.entry()?.key}</span> is stored as a workspace
						secret, and this deployment keeps a reference to it instead.
					</p>

					<form
						noValidate
						onSubmit={(e) => {
							e.preventDefault();
							void handleSubmit();
						}}
						class="flex flex-col gap-4"
					>
						<InputWithLabel label="Secret name" for="convert-secret-name">
							<Input
								id="convert-secret-name"
								name="convert-secret-name"
								class={needsName() ? "border-error!" : ""}
								disabled={submitting()}
								placeholder="e.g. Production database URL"
								type={InputType.Text}
								value={name()}
								onInput={(e) => {
									setName(e.currentTarget.value);
									setError("");
								}}
							/>
						</InputWithLabel>

						<InputWithLabel label="Value" for="convert-secret-value">
							<Input
								id="convert-secret-value"
								name="convert-secret-value"
								disabled={submitting()}
								placeholder="Secret value"
								type={InputType.Text}
								value={value()}
								onInput={(e) => setValue(e.currentTarget.value)}
							/>
						</InputWithLabel>

						<Show when={needsName()}>
							<Alert type="error" message="Give this secret a name" />
						</Show>
						<Show when={!needsName() && isTaken()}>
							<Alert
								type="warning"
								message={`A secret named "${name().trim()}" already exists in this workspace`}
							/>
						</Show>
						<Show when={error()}>
							<Alert type="error" message={error()} />
						</Show>

						<div class="flex justify-between gap-3 mt-2">
							<Button
								type="button"
								variant={ButtonVariant.Plain}
								onClick={handleClose}
								class="cursor-pointer"
							>
								Cancel
							</Button>
							<Button
								type="submit"
								variant={ButtonVariant.Contained}
								disabled={submitting() || value() === ""}
								class="cursor-pointer"
							>
								{submitting() ? "Converting…" : "Convert"}
							</Button>
						</div>
					</form>
				</ModalContainer>
			)}
		/>
	);
};

export default EnvConvertModal;
