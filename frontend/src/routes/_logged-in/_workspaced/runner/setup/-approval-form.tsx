import { useMutation } from "@tanstack/solid-query";
import { createSignal, Show } from "solid-js";
import { ApproveRunnerLinkRequest, ApproveRunnerLinkResponse, GetRunnerLinkResponse } from "~/bindings";
import { Alert, Button, ButtonVariant, Input, InputLabel, InputType, useToast } from "~/components";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { formatRelativeTime } from "~/utils/func";
import { MapView } from "./-map-view";

const DetailCell = (props: { label: string; value: string }) => (
	<div class="flex flex-col gap-1 min-w-0">
		<span class="text-grey/70 text-xxs uppercase tracking-wider">{props.label}</span>
		<span class="font-log text-white text-sm truncate" title={props.value}>
			{props.value}
		</span>
	</div>
);

export const ApprovalForm = (props: { link: GetRunnerLinkResponse; code: string; onApproved: () => void }) => {
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
		<form
			noValidate
			onSubmit={onApprove}
			class="mx-auto flex flex-col gap-6 w-full"
			style={{ "max-width": "40rem" }}
		>
			<Show when={props.link.latitude && props.link.longitude}>
				<MapView lat={props.link.latitude!} lng={props.link.longitude!} />
			</Show>

			<section class="grid grid-cols-2 gap-x-8 gap-y-4">
				<DetailCell label="Version" value={props.link.version} />
				<DetailCell label="Started" value={formatRelativeTime(props.link.createdAt as unknown as string)} />
				<DetailCell label="OS" value={props.link.os} />
				<DetailCell label="Architecture" value={props.link.arch} />
				<DetailCell label="Hostname" value={props.link.hostname} />
				<DetailCell
					label="Location"
					value={[props.link.city, props.link.country].filter(Boolean).join(", ") || "Unknown"}
				/>
				<DetailCell label="Public IP" value={props.link.publicIp} />
				<DetailCell label="Private IP" value={props.link.privateIp} />
			</section>

			<div class="h-px bg-border-color" />

			<div class="flex items-center gap-4">
				<InputLabel parentClass="flex-2" for="runner-name" label="Name" />
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
