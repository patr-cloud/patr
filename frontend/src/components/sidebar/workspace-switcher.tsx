import { Link as RouterLink } from "@tanstack/solid-router";
import { FiSettings } from "solid-icons/fi";
import { createMemo, createResource, createSignal, For, Show, Suspense } from "solid-js";
import { ListUserWorkspacesResponse } from "~/bindings";
import { Button, Initials, Link, useToast } from "~/components";
import { useAuthState, useClickOutside } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

const WorkspaceSwitcher = () => {
	const [authState, _] = useAuthState();
	const [workspaceId, setWorkspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const [workspaceRef, setWorkspaceRef] = createSignal<HTMLDivElement>();
	useClickOutside(workspaceRef, () => setShowSwitcher(false));

	const [showSwitcher, setShowSwitcher] = createSignal(false);

	const listWorkspacesParams = createMemo(() => {
		return [authState()] as const;
	});

	const [listWorkspacesResource] = createResource(listWorkspacesParams, async ([auth]) => {
		if (auth === null || auth.type !== "LoggedIn") {
			return { workspaces: [] };
		}
		const response = await httpRequest<ListUserWorkspacesResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/workspaces`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch workspaces:", response.data.error);
			toast("Failed to fetch workspaces", "error");
			return { workspaces: [] };
		}

		return response.data;
	});

	const currentWorkspaceInfo = () => {
		const workspaces = listWorkspacesResource.latest?.workspaces || [];
		return workspaces.find((ws) => ws.id === workspaceId());
	};

	return (
		<div class="relative select-none" ref={setWorkspaceRef}>
			<div
				class="flex justify-between items-center py-sm px-md cursor-pointer hover:bg-secondary-dark rounded-xs w-full br-sm bg-secondary-dark gap-xxs"
				onClick={() => setShowSwitcher(!showSwitcher())}
			>
				<div class="flex flex-row items-center justify-start w-full">
					<Suspense fallback={<div class="text-sm text-white">Loading...</div>}>
						<Initials
							firstName={() => currentWorkspaceInfo()?.name ?? ".."}
							size="lg"
							class="mr-3 bg-secondary!"
						/>
						<p class="text-sm text-white text-ellipsis overflow-hidden">
							{currentWorkspaceInfo() ? currentWorkspaceInfo()!.name : "Select A Workspace"}
						</p>
					</Suspense>
				</div>

				<RouterLink to="/workspace" class="text-xs text-gray-400">
					<FiSettings />
				</RouterLink>
			</div>

			<Show when={showSwitcher()}>
				<div
					class="absolute bottom-18 left-0 min-w-72 max-h-160 
          shadow-high rounded-xs z-10 bg-secondary-light text-white
          border border-border-color flex flex-col items-start justify-start py-md px-sm pb-0"
				>
					<p class="text-center w-full text-md mb-sm">Workspaces</p>

					<div class="w-full border border-border-color flex flex-col flex-1 max-h-80 min-h-0">
						<Suspense
							fallback={
								<div
									class="text-sm text-white text-center px-md py-sm
                 					 bg-secondary-medium! hover:border-primary w-full
                 					 cursor-pointer rounded-xs rounded-b-none
									border-b border-border-color"
								>
									Loading...
								</div>
							}
						>
							<div class="overflow-y-scroll flex flex-col justify-start items-start max-h-80 min-h-0 flex-1">
								<For each={listWorkspacesResource.latest?.workspaces || []}>
									{(workspace, index) => (
										<Button
											onClick={() => {
												console.log("Switching to workspace:", workspace.id);
												setShowSwitcher(false);
												setWorkspaceId(workspace.id);
											}}
											class={`px-sm py-sm bg-secondary-medium! hover:border-primary! cursor-pointer overflow-hidden rounded-xs w-full ${
												index() !== (listWorkspacesResource.latest?.workspaces.length || 0)
													? "ul-light"
													: ""
											} relative justify-start`}
										>
											<Initials firstName={workspace.name ?? ".."} size="sm" class="mr-3" />
											<p class="text-sm text-white text-ellipsis overflow-hidden">
												{workspace.name}
											</p>
										</Button>
									)}
								</For>
							</div>
						</Suspense>
					</div>
					<div class="px-md py-md text-center bg-secondary-light! cursor-pointer w-full">
						<Link href="/workspace/new" class="text-sm text-primary" external={false}>
							CREATE WORKSPACE
						</Link>
					</div>
				</div>
			</Show>
		</div>
	);
};

export default WorkspaceSwitcher;
