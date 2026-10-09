import { useQueryClient } from "@tanstack/solid-query";
import { createEffect, createMemo, createSignal, Show } from "solid-js";
import { UpdateRunnerRequest, UpdateRunnerResponse } from "~/bindings";
import {
	Button,
	ButtonVariant,
	CopyableField,
	CopyableFieldVariant,
	Input,
	InputType,
	Label,
	UnsavedChangesGuard,
	useToast,
} from "~/components";
import { createFormAction } from "~/hooks";
import { useGetPermissions } from "~/hooks/is-allowed";
import { useRunnerInfoQuery } from "~/hooks/fetch";
import { runnerKeys } from "~/hooks/query-keys";
import { formatRelativeTime } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";

interface RunnerInfoProps {
	runnerId: string;
}

const RunnerInfo = (props: RunnerInfoProps) => {
	const toast = useToast();
	const queryClient = useQueryClient();

	const runnerQuery = useRunnerInfoQuery(() => props.runnerId);
	const runnerPermissions = useGetPermissions("runner", () => props.runnerId);

	const [name, setName] = createSignal<string>();

	// Seed the editable name once the runner info loads. Only once: the query
	// refetches whenever the window regains focus, and reseeding then would wipe
	// out a rename the user is in the middle of typing.
	createEffect(() => {
		const runnerName = runnerQuery.data?.runner.name;
		if (runnerName !== undefined && name() === undefined) {
			setName(runnerName);
		}
	});

	const isDirty = createMemo(() => {
		const current = runnerQuery.data?.runner.name;
		return current !== undefined && (name() ?? "").trim() !== current;
	});

	const lastSeenText = () => {
		const runner = runnerQuery.data?.runner;
		if (!runner) return "";
		if (runner.connected) return "Connected now";
		return runner.lastSeen ? `Last seen ${formatRelativeTime(runner.lastSeen)}` : "Never connected";
	};

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId: wsId }) => {
		const runnerName = (name() ?? "").trim();

		if (!runnerName) {
			toast("Name is required", "error");
			return;
		}

		const requestBody: UpdateRunnerRequest = {
			name: runnerName,
		};

		const response = await httpRequest<UpdateRunnerResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner/${props.runnerId}`,
			{
				method: "PATCH",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			if (response.data.error === "resourceAlreadyExists") {
				toast("A runner with this name already exists", "error");
				return;
			}
			toast("Failed to update runner", "error");
			return;
		}

		setName(runnerName);
		await queryClient.invalidateQueries({ queryKey: runnerKeys.all(wsId) });
		toast("Runner updated successfully", "success");
	});

	return (
		<form noValidate onSubmit={onSubmit} class="flex flex-col gap-8 w-full">
			<div class="flex flex-col gap-4 w-full">
				<div class="flex gap-8 items-start w-full">
					<Label parentClass="flex-2 pt-2.5" for="runner-name" label="Runner" />
					<div class="flex-10 flex items-center gap-4 w-full">
						<CopyableField
							value={props.runnerId}
							variant={CopyableFieldVariant.Input}
							buttonPosition="start"
							class="flex-4 min-w-0"
						/>

						<Input
							id="runner-name"
							name="runner-name"
							class="flex-4"
							placeholder="Runner Name"
							type={InputType.Text}
							value={name() ?? ""}
							disabled={!runnerPermissions().edit}
							onInput={(e) => setName(e.currentTarget.value)}
						/>

						<Input class="flex-2" disabled={true} type={InputType.Text} value={lastSeenText()} />
					</div>
				</div>
			</div>

			<Show when={runnerPermissions().edit}>
				<div class="w-full flex justify-end">
					<Button
						variant={ButtonVariant.Contained}
						type="submit"
						disabled={!isDirty() || isLoading()}
						loading={isLoading()}
						loadingContent={() => <span>Saving...</span>}
					>
						Save Changes
					</Button>
				</div>
			</Show>

			<UnsavedChangesGuard
				when={isDirty}
				message="You have unsaved changes to this runner. If you leave now, they'll be lost."
			/>
		</form>
	);
};

export default RunnerInfo;
