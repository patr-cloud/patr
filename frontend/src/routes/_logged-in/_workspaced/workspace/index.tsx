import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import {
	Button,
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
import { useWorkspaceInfoQuery } from "~/hooks/fetch";
import { workspaceKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import { httpRequest } from "~/utils/http-request";
import { createEffect, createSignal } from "solid-js";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { EventT } from "~/utils/types";

const General = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const queryClient = useQueryClient();

	const workspaceInfoQuery = useWorkspaceInfoQuery();

	const [name, setName] = createSignal("");
	const [_hasUpdated, setHasUpdated] = createSignal(false);

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
			setHasUpdated(false);
			queryClient.setQueryData<GetWorkspaceInfoResponse>(workspaceKeys.info(id!), (prev) =>
				prev ? { ...prev, name: newName } : prev
			);
		} catch (error) {
			console.error("Failed to update workspace name:", error);
			toast("Failed to update workspace name", "error");
		}
	};

	return (
		<>
			<Title>Workspace Settings | Patr</Title>
			<PageContainer>
				<WorkspaceHeader workspaceName={workspaceInfoQuery.data?.name} activeTab="general" />
				<PageContainerBody class="flex flex-col gap-8">
					<form onSubmit={onSubmit} class="flex flex-col gap-6 justify-between w-full flex-1">
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
									name().trim() === (workspaceInfoQuery.data?.name ?? "") || name().trim() === ""
								}
							>
								Update
							</Button>
						</div>
					</form>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/workspace/")({
	component: General,
});
