import { createSignal } from "solid-js";
import { CreateWorkspaceResponse } from "~/bindings";
import {
	Button,
	ButtonVariant,
	Input,
	InputLabel,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";

const CreateWorkspace = () => {
	const [authState] = useAuthState();
	const [currentWorkspaceName, setCurrentWorkspaceName] = useLastWorkspaceId();
	const toast = useToast();

	const [workspaceName, setWorkspaceName] = createSignal("");

	const onCreateWorkspace = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();

		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to create a workspace", "error");
			return;
		}

		const requestBody = {
			name: workspaceName(),
		};

		const response = await httpRequest<CreateWorkspaceResponse>(`${import.meta.env.VITE_BASE_URL}/api/workspace`, {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
			},
			body: JSON.stringify(requestBody),
		});

		if (!response.ok) {
			console.error("Failed to create workspace:", response.data.error);
			toast("Failed to create workspace", "error");
			return;
		}

		toast("Workspace created successfully", "success");
		setWorkspaceName("");

		if (!currentWorkspaceName() && response.data.id) {
			setCurrentWorkspaceName(response.data.id);
		}
	};

	return (
		<PageContainer>
			<PageContainerHead
				breadcrumbs={[
					{
						label: "Workspace",
						url: "/workspace",
					},
					{
						label: "Create",
					},
				]}
				subText="Workspaces are a way to organize your projects, deployments, and resources."
			/>
			<PageContainerBody class="flex flex-col">
				<form class="flex flex-1 flex-col justify-between gap-8" onSubmit={onCreateWorkspace}>
					<div class="flex gap-4 items-center">
						<InputLabel for="workspace-name" label="Workspace Name" parentClass="flex-2" />
						<Input
							name="workspace-name"
							value={workspaceName()}
							onInput={(e) => setWorkspaceName(e.currentTarget.value)}
							placeholder="Enter Workspace Name"
							type="text"
							class="flex-10"
						/>
					</div>

					<div class="flex justify-end w-full">
						<Button variant={ButtonVariant.Contained}>Create Workspace</Button>
					</div>
				</form>
			</PageContainerBody>
		</PageContainer>
	);
};

export default CreateWorkspace;
