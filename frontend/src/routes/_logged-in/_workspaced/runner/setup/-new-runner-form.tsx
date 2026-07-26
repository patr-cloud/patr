import { useMutation } from "@tanstack/solid-query";
import { createSignal, Show } from "solid-js";
import { ApproveRunnerLinkRequest, ApproveRunnerLinkResponse } from "~/bindings";
import { Alert, Button, ButtonVariant, Input, Label, InputType, useToast } from "~/components";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

/**
 * "New runner" mode: name the machine and register it as a fresh runner. This
 * is the unchanged behaviour of the original consent flow, hitting the approve
 * endpoint which creates the runner, its role, and its service account.
 */
export const NewRunnerForm = (props: { code: string; onApproved: () => void }) => {
	const [workspaceId] = useLastWorkspaceId();

	const [runnerName, setRunnerName] = createSignal("");
	const [nameError, setNameError] = createSignal("");
	const toast = useToast();

	const approveMutation = useMutation(() => ({
		mutationFn: async (body: ApproveRunnerLinkRequest) => {
			const wsId = workspaceId();
			const response = await httpRequest<ApproveRunnerLinkResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner/link/${props.code}/approve`,
				{ method: "POST", body: JSON.stringify(body) }
			);
			if (!response.ok) {
				throw new Error(response.data.error ?? "Approval failed");
			}
			return response.data;
		},
		onSuccess: () => props.onApproved(),
		onError: (err: Error) => toast(err.message, "error"),
	}));

	const onApprove = (e: Event) => {
		e.preventDefault();
		const name = runnerName().trim();
		if (!name) {
			setNameError("Runner name is required.");
			return;
		}
		if (!workspaceId()) {
			toast("No workspace selected. Pick one in the sidebar.", "error");
			return;
		}
		approveMutation.mutate({ runnerName: name });
	};

	return (
		<form noValidate onSubmit={onApprove} class="flex flex-col gap-6">
			<div class="flex items-center gap-4">
				<Label parentClass="flex-2" for="runner-name" label="Name" />
				<div class="flex-10 flex flex-col gap-2">
					<Input
						id="runner-name"
						name="runner-name"
						type={InputType.Text}
						placeholder="Give this runner a name"
						value={runnerName()}
						onInput={(e) => {
							setRunnerName(e.currentTarget.value);
							setNameError("");
						}}
					/>
					<Show when={nameError()}>
						<Alert type="error" message={nameError()} />
					</Show>
				</div>
			</div>

			<div class="flex justify-end">
				<Button
					variant={ButtonVariant.Contained}
					type="submit"
					loading={approveMutation.isPending}
					loadingContent={() => <span>Approving...</span>}
				>
					Approve
				</Button>
			</div>
		</form>
	);
};
