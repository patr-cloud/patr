import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import {
	Button,
	ButtonVariant,
	CopyableField,
	Input,
	InputType,
	Label,
	PageContainer,
	PageContainerBody,
	useToast,
} from "~/components";
import WorkspaceHeader from "./-components/workspace-header";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useWorkspaceInfoQuery, useUserInfoQuery } from "~/hooks/fetch";
import { workspaceKeys, workspacesKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import { httpRequest } from "~/utils/http-request";
import { createEffect, createSignal, Show } from "solid-js";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { UpdateWorkspaceInfoRequest } from "~/bindings/UpdateWorkspaceInfoRequest";
import { EventT } from "~/utils/types";
import { Color } from "~/utils/color";

const General = () => {
	const [authState] = useAuthState();
	const [workspaceId, setLastWorkspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const queryClient = useQueryClient();

	const workspaceInfoQuery = useWorkspaceInfoQuery();
	const userInfoQuery = useUserInfoQuery();
	const navigate = useNavigate();

	const [name, setName] = createSignal("");
	const [_hasUpdated, setHasUpdated] = createSignal(false);
	const [isUpdating, setIsUpdating] = createSignal(false);
	const [isConfirmingLeave, setIsConfirmingLeave] = createSignal(false);
	const [isLeaving, setIsLeaving] = createSignal(false);

	// The owner (super admin) cannot leave their own workspace.
	const canLeave = () =>
		!!userInfoQuery.data?.id &&
		!!workspaceInfoQuery.data?.superAdminId &&
		userInfoQuery.data.id !== workspaceInfoQuery.data.superAdminId;

	const onLeave = async () => {
		const id = workspaceId();
		if (!id || isLeaving()) return;
		setIsLeaving(true);
		try {
			const response = await httpRequest(`${import.meta.env.VITE_BASE_URL}/api/workspace/${id}/leave`, {
				method: "POST",
			});

			if (!response.ok) {
				console.error("Failed to leave workspace:", response.data);
				toast("Failed to leave workspace", "error");
				return;
			}

			toast("You've left the workspace", "success");
			setLastWorkspaceId(null);
			await queryClient.invalidateQueries({ queryKey: workspacesKeys.list() });
			navigate({ to: "/", replace: true });
		} catch (error) {
			console.error("Failed to leave workspace:", error);
			toast("Failed to leave workspace", "error");
		} finally {
			setIsLeaving(false);
		}
	};

	createEffect(() => {
		const info = workspaceInfoQuery.data;
		if (info?.name) {
			setName(info.name);
		}
	});

	const onSubmit = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();

		const auth = authState();
		const id = workspaceId();

		if (!auth || auth.type !== "LoggedIn" || id === "") {
			toast("You must be logged in to update the workspace", "error");
			return;
		}

		const newName = name().trim();
		if (!newName) {
			toast("Please enter a workspace name", "error");
			return;
		}

		if (isUpdating()) return;
		setIsUpdating(true);
		try {
			const body: UpdateWorkspaceInfoRequest = { name: newName };
			const response = await httpRequest(`${import.meta.env.VITE_BASE_URL}/api/workspace/${id}`, {
				method: "PATCH",
				body: JSON.stringify(body),
			});

			if (!response.ok) {
				console.error("Failed to update workspace name:", response.data);
				toast("Failed to update workspace name", "error");
				return;
			}

			toast("Workspace name updated successfully", "success");
			setHasUpdated(false);
			queryClient.setQueryData<GetWorkspaceInfoResponse>(workspaceKeys.info(id!), (prev) =>
				prev ? { ...prev, name: newName } : prev
			);
			await queryClient.invalidateQueries({ queryKey: workspacesKeys.list() });
		} catch (error) {
			console.error("Failed to update workspace name:", error);
			toast("Failed to update workspace name", "error");
		} finally {
			setIsUpdating(false);
		}
	};

	return (
		<>
			<Title>Workspace Settings | Patr</Title>
			<PageContainer>
				<WorkspaceHeader workspaceName={workspaceInfoQuery.data?.name} activeTab="general" />
				<PageContainerBody class="flex flex-col gap-8">
					<form onSubmit={onSubmit} class="flex flex-col gap-6 w-full">
						<div class="flex flex-col gap-4 items-start w-full">
							<div class="flex gap-8 items-center w-full">
								<Label parentClass="flex-2" label="Workspace ID" />
								<CopyableField
									value={workspaceId() || "Loading..."}
									class="flex-10"
									buttonPosition="start"
								/>
							</div>
							<div class="flex gap-8 items-center w-full">
								<Label parentClass="flex-2" for="workspace-name" label="Workspace Name" />
								<Input
									value={name()}
									onInput={(e) => {
										setHasUpdated(true);
										setName(e.currentTarget.value);
									}}
									class="flex-10"
									id="workspace-name"
									name="workspace-name"
									placeholder="Workspace Name"
									type={InputType.Text}
								/>
							</div>
						</div>

						<div class="w-full flex justify-end items-center">
							<Button
								type="submit"
								variant="contained"
								disabled={
									isUpdating() ||
									name().trim() === (workspaceInfoQuery.data?.name ?? "") ||
									name().trim() === ""
								}
							>
								Update
							</Button>
						</div>
					</form>

					<Show when={canLeave()}>
						<div class="flex gap-8 items-start w-full">
							<Label parentClass="flex-2" label="Leave Workspace" />
							<div class="flex-10 flex flex-col gap-3 items-start">
								<p class="text-grey text-sm">
									You'll lose access to this workspace and all its resources. You will need to be
									invited again to join this workspace.
								</p>
								<Show
									when={isConfirmingLeave()}
									fallback={
										<Button
											variant={ButtonVariant.Outlined}
											color={Color.Error}
											onClick={() => setIsConfirmingLeave(true)}
										>
											Leave workspace
										</Button>
									}
								>
									<div class="flex items-center gap-2">
										<Button
											variant={ButtonVariant.Contained}
											color={Color.Error}
											disabled={isLeaving()}
											loading={isLeaving()}
											loadingContent={() => <span>Leaving...</span>}
											onClick={() => onLeave()}
										>
											Confirm leave
										</Button>
										<Button
											variant={ButtonVariant.Outlined}
											onClick={() => setIsConfirmingLeave(false)}
										>
											Cancel
										</Button>
									</div>
								</Show>
							</div>
						</div>
					</Show>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/workspace/")({
	component: General,
});
