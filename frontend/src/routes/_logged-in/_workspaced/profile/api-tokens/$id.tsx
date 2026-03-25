import { createFileRoute } from "@tanstack/solid-router";
import { useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, createMemo, createResource, createSignal, For, Show, Suspense } from "solid-js";
import {
	GetApiTokenInfoResponse,
	ListUserWorkspacesResponse,
	UpdateApiTokenRequest,
	WorkspacePermission,
} from "~/bindings";
import {
	Button,
	ButtonVariant,
	DeleteModal,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import Input, { InputType } from "~/components/input";
import InputLabel from "~/components/input-label";
import { useAuthState } from "~/hooks";
import { useUserInfo } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import RegenerateModal from "./-components/regenerate-modal";
import { RegenerateApiTokenResponse } from "~/bindings/RegenerateApiTokenResponse";
import ApiTokenModal from "./-components/api-token-modal";
import WorkspacePermissionItem from "./-components/workspace-permission-item";

const ApiTokenInfo = () => {
	const [authState] = useAuthState();
	const userInfo = useUserInfo();
	const toast = useToast();
	const navigate = useNavigate();
	const params = Route.useParams();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);
	const [isRegenerateModalOpen, setIsRegenerateModalOpen] = createSignal(false);
	const [isApiTokenModalOpen, setIsApiTokenModalOpen] = createSignal(false);
	const [newApiToken, setNewApiToken] = createSignal<string>("");

	if (!params().id) {
		return <div>Invalid API Token ID</div>;
	}

	const fetchParams = createMemo(() => {
		return [authState()] as const;
	});

	const [apiTokenInfo] = createResource(fetchParams, async ([auth]) => {
		if (!auth || auth.type !== "LoggedIn") {
			return undefined;
		}

		const response = await httpRequest<GetApiTokenInfoResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/api-token/${params().id}`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch API Token Info:", response.data.error);
			toast("Failed to fetch API Token Info", "error");
			return undefined;
		}

		return { ...response.data };
	});

	const [workspaces] = createResource(authState, async (auth) => {
		if (!auth || auth.type !== "LoggedIn") {
			return { workspaces: [] };
		}

		const response = await httpRequest<ListUserWorkspacesResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/workspaces`,
			{ method: "GET" }
		);

		if (!response.ok) {
			console.error("Failed to fetch workspaces:", response.data.error);
			toast("Failed to fetch workspaces", "error");
			return { workspaces: [] };
		}

		return response.data;
	});

	// Permission editing state
	const [enabledWorkspaces, setEnabledWorkspaces] = createSignal<Set<string>>(new Set());
	const [workspacePermissions, setWorkspacePermissions] = createSignal<{
		[workspaceId: string]: WorkspacePermission;
	}>({});
	const [initialized, setInitialized] = createSignal(false);

	// Initialize permission state from fetched token info
	createEffect(() => {
		const info = apiTokenInfo();
		if (!info?.permissions || initialized()) return;

		const enabled = new Set<string>();
		const perms: { [key: string]: WorkspacePermission } = {};

		Object.entries(info.permissions).forEach(([wsId, perm]) => {
			if (perm) {
				enabled.add(wsId);
				perms[wsId] = perm;
			}
		});

		setEnabledWorkspaces(enabled);
		setWorkspacePermissions(perms);
		setInitialized(true);
	});

	const handleWorkspaceToggle = (workspaceId: string, enabled: boolean) => {
		const newEnabled = new Set(enabledWorkspaces());
		if (enabled) {
			newEnabled.add(workspaceId);
		} else {
			newEnabled.delete(workspaceId);
			const newPerms = { ...workspacePermissions() };
			delete newPerms[workspaceId];
			setWorkspacePermissions(newPerms);
		}
		setEnabledWorkspaces(newEnabled);
	};

	const handlePermissionChange = (workspaceId: string, permission: WorkspacePermission) => {
		setWorkspacePermissions((prev) => ({ ...prev, [workspaceId]: permission }));
	};

	const onClickDelete = async (e: EventT<MouseEvent, HTMLButtonElement>) => {
		e.preventDefault();

		const auth = authState();

		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to delete an API Token", "error");
			return;
		}

		const response = await httpRequest<void>(`${import.meta.env.VITE_BASE_URL}/api/user/api-token/${params().id}`, {
			method: "DELETE",
		});

		if (!response.ok) {
			console.error("Failed to delete API Token:", response.data.error);
			toast("Failed to delete API Token", "error");
			return;
		}

		toast("API Token deleted successfully", "success");
		navigate({ to: "/profile/api-tokens" });
	};

	const onClickRegenerate = async (e: EventT<MouseEvent, HTMLButtonElement>) => {
		e.preventDefault();

		const auth = authState();

		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to regenerate an API Token", "error");
			return;
		}

		const response = await httpRequest<RegenerateApiTokenResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/api-token/${params().id}/regenerate`,
			{
				method: "POST",
			}
		);

		if (!response.ok) {
			console.error("Failed to regenerate API Token:", response.data.error);
			toast("Failed to regenerate API Token", "error");
			return;
		}

		toast("API Token regenerated successfully", "success");
		setNewApiToken(response.data.token);
		setIsRegenerateModalOpen(false);
		setIsApiTokenModalOpen(true);
	};

	const onSavePermissions = async () => {
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to update an API Token", "error");
			return;
		}

		const info = apiTokenInfo();
		const body = {
			permissions: workspacePermissions(),
			tokenNbf: info?.tokenNbf,
			tokenExp: info?.tokenExp,
			allowedIps: info?.allowedIps || [],
		} as UpdateApiTokenRequest;

		const response = await httpRequest<null>(`${import.meta.env.VITE_BASE_URL}/api/user/api-token/${params().id}`, {
			method: "PATCH",
			body: JSON.stringify(body),
		});

		if (!response.ok) {
			console.error("Failed to update API Token:", response.data.error);
			toast("Failed to update API Token", "error");
			return;
		}

		toast("API Token permissions updated successfully", "success");
	};

	return (
		<>
			<Title>API Token Details | Patr</Title>
			<PageContainer>
				<Suspense fallback={<div>Loading API Token Info...</div>}>
					<PageContainerHead
						breadcrumbs={[
							{
								label: "API Tokens",
								url: "/profile/api-tokens",
							},
							{
								label: apiTokenInfo()?.name || "",
							},
						]}
						subText="Manage API Token here"
						actions={() => (
							<div class="flex gap-2 px-2">
								<RegenerateModal
									title="Regenerate API Token"
									onClickRegenerate={onClickRegenerate}
									resourceName={apiTokenInfo()?.name || ""}
									isOpen={isRegenerateModalOpen}
									setIsOpen={setIsRegenerateModalOpen}
								/>
								<DeleteModal
									title="Delete API Token"
									onClickDelete={onClickDelete}
									resourceName={apiTokenInfo()?.name || ""}
									isOpen={isDeleteModalOpen}
									setIsOpen={setIsDeleteModalOpen}
								/>
							</div>
						)}
					/>
					<PageContainerBody class="flex flex-col gap-8">
						<div class="flex flex-col gap-4 items-start w-full">
							<div class="flex gap-8 items-center w-full">
								<InputLabel parentClass="flex-2" for="deployment-id" label="ID" />
								<Input
									value={apiTokenInfo()?.id || ""}
									disabled={true}
									class="flex-10"
									name="deployment-id"
									placeholder="Deployment ID"
									type={InputType.Text}
								/>
							</div>

							<div class="flex gap-8 items-center w-full">
								<InputLabel parentClass="flex-2" for="deployment-name" label="Name" />
								<Input
									value={apiTokenInfo()?.name || ""}
									class="flex-10"
									name="deployment-name"
									placeholder="Deployment Name"
									type={InputType.Text}
									disabled={true}
								/>
							</div>
						</div>

						{/* Workspace Permissions Section */}
						<Suspense
							fallback={
								<div class="flex items-center justify-center py-8">
									<div class="text-gray-400">Loading workspaces...</div>
								</div>
							}
						>
							<Show when={initialized()}>
								<div class="flex flex-col gap-4 items-start w-full">
									<div class="flex justify-between items-center w-full">
										<h3 class="text-lg text-white">Workspace Permissions</h3>
										<Button variant={ButtonVariant.Contained} onClick={onSavePermissions}>
											Save Permissions
										</Button>
									</div>

									<For
										each={workspaces.latest?.workspaces || []}
										fallback={<div class="text-gray-400">No workspaces available</div>}
									>
										{(ws) => (
											<WorkspacePermissionItem
												workspace={ws}
												isSuperAdmin={userInfo()?.id === ws.superAdminId}
												enabled={enabledWorkspaces().has(ws.id)}
												initialPermission={apiTokenInfo()?.permissions?.[ws.id]}
												onToggle={handleWorkspaceToggle}
												onPermissionChange={handlePermissionChange}
											/>
										)}
									</For>
								</div>
							</Show>
						</Suspense>
					</PageContainerBody>
				</Suspense>

				<ApiTokenModal
					isOpen={isApiTokenModalOpen}
					setIsOpen={setIsApiTokenModalOpen}
					token={newApiToken}
					onClose={() => navigate({ to: "/profile/api-tokens" })}
				/>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/profile/api-tokens/$id")({
	component: ApiTokenInfo,
});
