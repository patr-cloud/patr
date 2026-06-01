import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, Show } from "solid-js";
import { CreateWorkspaceResponse } from "~/bindings";
import {
	Alert,
	Button,
	ButtonVariant,
	Input,
	Label,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import { createAsyncAction, useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { workspacesKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import { httpRequest } from "~/utils/http-request";

const CreateWorkspace = () => {
	const [authState] = useAuthState();
	const [, setCurrentWorkspaceName] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const queryClient = useQueryClient();

	const [workspaceName, setWorkspaceName] = createSignal("");
	const [nameError, setNameError] = createSignal("");

	const { execute: createWorkspace, isLoading } = createAsyncAction(async () => {
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to create a workspace", "error");
			return;
		}

		const name = workspaceName().trim();
		if (!name) {
			setNameError("Workspace name is required.");
			return;
		}

		const response = await httpRequest<CreateWorkspaceResponse>(`${import.meta.env.VITE_BASE_URL}/api/workspace`, {
			method: "POST",
			body: JSON.stringify({ name }),
		});

		if (!response.ok) {
			setNameError("Failed to create workspace. Please try a different name.");
			return;
		}

		toast("Workspace created successfully", "success");

		// Always switch to the new workspace. The previous guard
		// `!currentWorkspaceName()` was dead code — by the time we're on
		// `/workspace/new` the lastWorkspaceId cookie is always set (the
		// `_workspaced` layout requires it), so the cookie was never updated
		// and the user stayed on the old workspace.
		if (response.data.id) {
			setCurrentWorkspaceName(response.data.id);
			toast(`Switched to ${name}`, "success");
		}

		await queryClient.invalidateQueries({ queryKey: workspacesKeys.list() });

		navigate({ to: "/workspace" });
	});

	return (
		<>
			<Title>New Workspace | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Workspace",
							url: "/workspace",
						},
						{
							label: "New",
						},
					]}
					subText="Workspaces are a way to organize your projects, deployments, and resources."
				/>
				<PageContainerBody class="flex flex-col">
					<form
						noValidate
						class="flex flex-col gap-8"
						onSubmit={async (e: SubmitEvent) => {
							e.preventDefault();
							await createWorkspace().catch(() => {
								toast("An unexpected error occurred while creating the workspace", "error");
							});
						}}
					>
						<div class="flex gap-8 items-center w-full">
							<Label for="workspace-name" label="Workspace Name" parentClass="flex-2" />
							<div class="flex-10 flex flex-col">
								<Input
									id="workspace-name"
									name="workspace-name"
									value={workspaceName()}
									onInput={(e) => {
										setWorkspaceName(e.currentTarget.value);
										setNameError("");
									}}
									placeholder="Enter Workspace Name"
									type="text"
								/>
								<Show when={nameError()}>
									<div class="mt-1">
										<Alert message={nameError()} type="error" />
									</div>
								</Show>
							</div>
						</div>

						<div class="flex justify-end w-full">
							<Button
								type="submit"
								loading={isLoading}
								loadingContent={() => <span>Creating Workspace...</span>}
								variant={ButtonVariant.Contained}
							>
								Create Workspace
							</Button>
						</div>
					</form>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/workspace_/new")({
	component: CreateWorkspace,
});
