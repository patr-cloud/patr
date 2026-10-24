import { useQueryClient } from "@tanstack/solid-query";
import { createEffect, createMemo, createSignal, Show } from "solid-js";
import { lt } from "semver";
import { FiAlertTriangle } from "solid-icons/fi";
import { UpdateRunnerRequest, UpdateRunnerResponse } from "~/bindings";
import {
	Alert,
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
import { useApiVersionQuery, useRunnerInfoQuery } from "~/hooks/fetch";
import { runnerKeys } from "~/hooks/query-keys";
import { formatRelativeTime } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";

interface RunnerInfoProps {
	runnerId: string;
	reconnectCommand: string;
}

const RunnerInfo = (props: RunnerInfoProps) => {
	const toast = useToast();
	const queryClient = useQueryClient();

	const runnerQuery = useRunnerInfoQuery(() => props.runnerId);
	const runnerPermissions = useGetPermissions("runner", () => props.runnerId);
	const versionQuery = useApiVersionQuery();

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

	const neverConnected = () => {
		const runner = runnerQuery.data?.runner;
		return !!runner && !runner.connected && !runner.lastSeen;
	};

	// '0.0.0' is the backfill sentinel for rows that existed before version
	// tracking or never reported a version.
	const versionUnknown = () => runnerQuery.data?.runner.version === "0.0.0";
	// Only flag outdated once we know the runner has actually connected at
	// least once — fresh-never-connected rows sit at the sentinel and there's
	// nothing for the operator to act on.
	const outdated = () => {
		const runner = runnerQuery.data?.runner;
		const apiVersion = versionQuery.data?.version;
		return !!runner && !versionUnknown() && !!runner.lastSeen && !!apiVersion && lt(runner.version, apiVersion);
	};

	const versionText = () => {
		const version = runnerQuery.data?.runner.version;
		if (!version) return "";
		if (versionUnknown()) return "Unknown";
		return outdated() ? `${version} (update available)` : version;
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
		<form noValidate onSubmit={onSubmit} class="flex flex-col gap-6 w-full">
			<Show when={neverConnected()}>
				<div class="flex flex-col gap-sm border border-border-color rounded-xs p-md">
					<Alert type="warning" message="This runner hasn't connected yet." />
					<p class="text-grey text-sm">
						To set it up, run this on the machine that should host it. The machine needs Linux with systemd,
						and Docker.
					</p>
					<CopyableField value={props.reconnectCommand} innerClass="font-mono" />
					<Show when={runnerPermissions().regenerateToken}>
						<p class="text-grey text-sm">
							If the Patr CLI on that machine isn't logged in, use Regenerate Token above and paste the
							token when it asks.
						</p>
					</Show>
				</div>
			</Show>

			<div class="flex flex-col gap-4 w-full">
				<div class="flex gap-8 items-start w-full">
					<Label parentClass="flex-2 pt-2.5" label="ID" />
					<div class="flex-10 min-w-0">
						<CopyableField
							value={props.runnerId}
							variant={CopyableFieldVariant.Input}
							buttonPosition="start"
							class="w-full"
						/>
					</div>
				</div>

				<div class="flex gap-8 items-start w-full">
					<Label parentClass="flex-2 pt-2.5" for="runner-name" label="Name" />
					<div class="flex-10">
						<Input
							id="runner-name"
							name="runner-name"
							class="w-full"
							placeholder="Runner Name"
							type={InputType.Text}
							value={name() ?? ""}
							disabled={!runnerPermissions().edit}
							onInput={(e) => setName(e.currentTarget.value)}
						/>
					</div>
				</div>

				<div class="flex gap-8 items-start w-full">
					<Label parentClass="flex-2 pt-2.5" for="runner-version" label="Version" />
					<div class="flex-10 flex flex-col gap-xs">
						<Input
							id="runner-version"
							name="runner-version"
							class="w-full"
							disabled={true}
							type={InputType.Text}
							value={versionText()}
							innerClass={outdated() ? "disabled:text-warning" : ""}
							startIcon={
								outdated()
									? () => (
											<FiAlertTriangle
												class="text-warning shrink-0 ml-md mr-sm"
												size={14}
												aria-label="Update available"
											/>
										)
									: undefined
							}
						/>
						<Show when={outdated()}>
							<p class="text-xs text-grey">
								Run <code class="text-white font-log">patr upgrade</code> on the runner host to update.
							</p>
						</Show>
					</div>
				</div>

				<div class="flex gap-8 items-start w-full">
					<Label parentClass="flex-2 pt-2.5" label="Last Seen" />
					<div class="flex-10">
						<Input class="w-full" disabled={true} type={InputType.Text} value={lastSeenText()} />
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
