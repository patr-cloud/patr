import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { Show } from "solid-js";
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
import { useCreateWorkspace } from "~/hooks/use-create-workspace";
import { cloudOnly } from "~/utils/env";

const CreateWorkspace = () => {
	const toast = useToast();
	const navigate = useNavigate();

	const { workspaceName, setWorkspaceName, nameError, setNameError, isLoading, submit } = useCreateWorkspace({
		onCreated: (_id, name) => {
			toast(`Switched to ${name}`, "success");
			navigate({ to: "/workspace" });
		},
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
							await submit().catch(() => {
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

export const Route = createFileRoute("/_logged-in/_workspaced/workspace_/new")(
	cloudOnly({
		component: CreateWorkspace,
	})
);
