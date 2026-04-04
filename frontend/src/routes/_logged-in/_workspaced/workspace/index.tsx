import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { Button, CopyableField, PageContainer, PageContainerBody, useToast } from "~/components";
import Input, { InputType } from "~/components/input";
import InputLabel from "~/components/input-label";
import WorkspaceHeader from "./-components/workspace-header";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { createEffect, createResource, createSignal } from "solid-js";
import { GetWorkspaceInfoResponse } from "~/bindings/GetWorkspaceInfoResponse";
import { EventT } from "~/utils/types";

const General = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const resourceParamsWorkspace = () => {
		return [authState(), workspaceId()] as const;
	};

	const [name, setName] = createSignal("");
	const [_hasUpdated, setHasUpdated] = createSignal(false);

	const [workspaceInfo, { mutate: mutateWorkspaceInfo }] = createResource(
		resourceParamsWorkspace,
		async ([auth, id]) => {
			if (!auth || auth.type !== "LoggedIn" || id === "") {
				return;
			}
			const response = await httpRequest<GetWorkspaceInfoResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${id}`,
				{
					method: "GET",
				}
			);
			if (!response.ok) {
				console.error("Failed to fetch workspace info:", response.data.error);
				toast("Failed to fetch workspace info", "error");
				return undefined;
			}
			return response.data;
		}
	);

	createEffect(() => {
		const info = workspaceInfo();
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
			mutateWorkspaceInfo((prev) => (prev ? { ...prev, name: newName } : prev));
		} catch (error) {
			console.error("Failed to update workspace name:", error);
			toast("Failed to update workspace name", "error");
		}
	};

	return (
		<>
			<Title>Workspace Settings | Patr</Title>
			<PageContainer>
				<WorkspaceHeader workspaceName={workspaceInfo()?.name} activeTab="general" />
				<PageContainerBody class="flex flex-col gap-8">
					<form onSubmit={onSubmit} class="flex flex-col gap-6 justify-between w-full flex-1">
						<div class="flex flex-col gap-4 items-start w-full">
							<div class="flex gap-8 items-center w-full">
								<InputLabel parentClass="flex-2" label="Workspace ID" />
								<CopyableField
									value={workspaceId() || "Loading..."}
									class="flex-10"
									buttonPosition="start"
								/>
							</div>
							<div class="flex gap-8 items-center w-full">
								<InputLabel parentClass="flex-2" for="workspace-name" label="Workspace Name" />
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
								disabled={name().trim() === (workspaceInfo()?.name ?? "") || name().trim() === ""}
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
