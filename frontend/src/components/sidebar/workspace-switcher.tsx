import { Link as RouterLink } from "@tanstack/solid-router";
import { FiSettings } from "solid-icons/fi";
import { createSignal, For, Show } from "solid-js";
import { Button, Initials, Link } from "~/components";
import { useClickOutside } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useWorkspacesQuery } from "~/hooks/fetch";

const WorkspaceSwitcher = () => {
	const [workspaceId, setWorkspaceId] = useLastWorkspaceId();
	const [workspaceRef, setWorkspaceRef] = createSignal<HTMLDivElement>();
	useClickOutside(workspaceRef, () => setShowSwitcher(false));

	const [showSwitcher, setShowSwitcher] = createSignal(false);

	const workspacesQuery = useWorkspacesQuery();

	const currentWorkspaceInfo = () => {
		const workspaces = workspacesQuery.data?.workspaces || [];
		return workspaces.find((ws) => ws.id === workspaceId());
	};

	return (
		<div class="relative select-none" ref={setWorkspaceRef}>
			<div
				class="flex justify-between items-center py-sm px-md cursor-pointer hover:bg-secondary-dark rounded-xs w-full br-sm bg-secondary-dark gap-xxs"
				onClick={() => setShowSwitcher(!showSwitcher())}
			>
				<div class="flex flex-row items-center justify-start w-full">
					<Initials
						firstName={() => currentWorkspaceInfo()?.name ?? ".."}
						size="lg"
						class="mr-3 bg-secondary!"
					/>
					<p class="text-sm text-white text-ellipsis overflow-hidden">
						{currentWorkspaceInfo() ? currentWorkspaceInfo()!.name : "Select A Workspace"}
					</p>
				</div>

				<RouterLink to="/workspace" class="text-xs text-gray-400">
					<FiSettings />
				</RouterLink>
			</div>

			<Show when={showSwitcher()}>
				<div
					class="absolute bottom-18 left-0 min-w-72 max-w-[calc(100vw-1rem)] max-h-160
          shadow-high rounded-xs z-50 bg-secondary-light text-white
          border border-border-color flex flex-col items-start justify-start py-md px-sm pb-0"
				>
					<p class="text-center w-full text-md mb-sm">Workspaces</p>

					<div class="w-full border border-border-color flex flex-col flex-1 max-h-80 min-h-0">
						<div class="overflow-y-scroll flex flex-col justify-start items-start max-h-80 min-h-0 flex-1">
							<For each={workspacesQuery.data?.workspaces || []}>
								{(workspace, index) => (
									<Button
										onClick={() => {
											console.log("Switching to workspace:", workspace.id);
											setShowSwitcher(false);
											setWorkspaceId(workspace.id);
										}}
										class={`px-sm py-sm bg-secondary-medium! hover:border-primary! cursor-pointer overflow-hidden rounded-xs w-full ${
											index() !== (workspacesQuery.data?.workspaces.length || 0) ? "ul-light" : ""
										} relative justify-start`}
									>
										<Initials firstName={workspace.name ?? ".."} size="sm" class="mr-3" />
										<p class="text-sm text-white text-ellipsis overflow-hidden">{workspace.name}</p>
									</Button>
								)}
							</For>
						</div>
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
