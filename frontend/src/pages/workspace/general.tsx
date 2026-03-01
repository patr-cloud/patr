import {
	Button,
	ButtonVariant,
	Input,
	InputLabel,
	InputType,
	PageContainer,
	PageContainerBody,
	useToast,
} from "~/components";
import WorkspaceHeader from "./workspace-header";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { createResource, createSignal } from "solid-js";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";

const General = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const resourceParamsWorkspace = () => {
		return [authState(), workspaceId()] as const;
	};

	const [name, setName] = createSignal("");

	const [workspaceInfo] = createResource(resourceParamsWorkspace, async ([auth, id]) => {
		if (!auth || auth.type !== "LoggedIn" || id === "") {
			return;
		}
		const response = await httpRequest<GetWorkspaceInfoResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${id}`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);
		if (!response.ok) {
			console.error("Failed to fetch workspace info:", response.data.error);
			toast("Failed to fetch workspace info", "error");
			return undefined;
		}
		return response.data;
	});
	const onSubmit = async (e: SubmitEvent) => {
		e.preventDefault();

		const auth = authState();
		const id = workspaceId();

		if (!auth || auth.type !== "LoggedIn" || id === "") {
			toast("You must be logged in to update the workspace", "error");
			return;
		}

		const newName = name().trim();
		if (!newName) {
			toast("Please enter a new workspace name", "error");
			return;
		}

		try {
			const response = await httpRequest(`${import.meta.env.VITE_BASE_URL}/api/workspace/${id}`, {
				method: "PATCH",
				body: JSON.stringify({ name: newName }),
			});

			if (!response.ok) {
				console.error("Failed to update workspace name:", response.data);
				toast("Failed to update workspace name", "error");
				return;
			}

			toast("Workspace name updated successfully", "success");
			setName("");
		} catch (error) {
			console.error("Failed to update workspace name:", error);
			toast("Failed to update workspace name", "error");
		}
	};
	return (
		<>
			<PageContainer>
				<WorkspaceHeader workspaceName={workspaceInfo()?.name} activeTab="general" />
				<PageContainerBody class="flex flex-col justify-between gap-8">
					<form onSubmit={onSubmit} class="flex flex-col gap-8 items-start w-full justify-between flex-1">
						<div class="flex w-full flex-col justify-between gap-6 h-full flex-1">
							<div class="flex flex-col gap-6 items-start w-full">
								<div class="flex gap-8 items-center w-full">
									<InputLabel parentClass="flex-2" for="workspace-name" label="Workspace Name" />
									<Input
										value={workspaceInfo()?.name}
										class="flex-10"
										name="workspace-name"
										placeholder="Workspace Current Name"
										type={InputType.Text}
										disabled={true}
									/>
								</div>
								<div class="flex gap-8 items-center w-full">
									<InputLabel parentClass="flex-2" for="new-workspace-name" label="New Workspace Name" />
									<Input
										value={name()}
										onInput={(e) => {
											setName(e.currentTarget.value);
										}}
										class="flex-10"
										name="new-workspace-name"
										placeholder="Enter Workspace New Name"
										type={InputType.Text}
									/>
								</div>
							</div>
						</div>

						<div class="w-full flex justify-end">
							<Button variant={ButtonVariant.Contained} type="submit">
								Change Name
							</Button>
						</div>
					</form>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export default General;
