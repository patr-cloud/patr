import { createFileRoute } from "@tanstack/solid-router";
import { useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, createSignal, For, Show, Suspense } from "solid-js";
import { GetApiTokenInfoResponse, UpdateApiTokenRequest } from "~/bindings";
import { WorkspacePermission } from "~/utils/types";
import {
	Button,
	ButtonVariant,
	DeleteModal,
	Input,
	InputType,
	InputWithLabel,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import { useQueryClient } from "@tanstack/solid-query";
import { useAuthState } from "~/hooks";
import { useUserInfo } from "~/hooks/state-hooks";
import { useApiTokenInfoQuery, useWorkspacesQuery } from "~/hooks/fetch";
import { apiTokenKeys } from "~/hooks/query-keys";
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
	const queryClient = useQueryClient();
	const params = Route.useParams();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);
	const [isRegenerateModalOpen, setIsRegenerateModalOpen] = createSignal(false);
	const [isApiTokenModalOpen, setIsApiTokenModalOpen] = createSignal(false);
	const [newApiToken, setNewApiToken] = createSignal<string>("");

	const apiTokenInfoQuery = useApiTokenInfoQuery(() => params().id);
	const workspacesQuery = useWorkspacesQuery();

	const apiTokenInfo = () => apiTokenInfoQuery.data;

	// Token-name draft. Seeded once from the query when data arrives; user
	// edits drive the input. Saving sends the full token object (see saveToken).
	const [tokenName, setTokenName] = createSignal<string | undefined>();
	const [isSaving, setIsSaving] = createSignal(false);

	createEffect(() => {
		const persisted = apiTokenInfo()?.name;
		if (persisted !== undefined && tokenName() === undefined) {
			setTokenName(persisted);
		}
	});

	// Permission editing state
	const [enabledWorkspaces, setEnabledWorkspaces] = createSignal<Set<string>>(new Set());
	const [workspacePermissions, setWorkspacePermissions] = createSignal<{
		[workspaceId: string]: WorkspacePermission;
	}>({});
	const [initialized, setInitialized] = createSignal(false);

	// Initialize permission state from fetched token info. We treat any
	// successful query as initialised — even if `permissions` is omitted from
	// the response (empty BTreeMap), the section should render so the user
	// can tick a workspace and assign perms from scratch.
	createEffect(() => {
		const info = apiTokenInfo();
		if (!info || initialized()) return;

		const enabled = new Set<string>();
		const perms: { [key: string]: WorkspacePermission } = {};

		// The wire shape is superAdminOf + role grants now; this screen still
		// edits the old per-permission shape until the token-screen rework, so
		// member workspaces surface only as "member" with no detail.
		for (const wsId of info.superAdminOf ?? []) {
			enabled.add(wsId);
			perms[wsId] = { type: "superAdmin" };
		}
		for (const wsId of Object.keys(info.grants ?? {})) {
			enabled.add(wsId);
			perms[wsId] = { type: "member" } as WorkspacePermission;
		}

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

	// Single save for the whole token: the full object (name + permissions +
	// carried nbf/exp/allowedIps) is sent on every update. Both the name form
	// and the permissions section call this, so neither clobbers the other.
	const saveToken = async () => {
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to update an API Token", "error");
			return;
		}

		const info = apiTokenInfo();
		const name = (tokenName() ?? info?.name ?? "").trim();
		if (!name) {
			toast("Token name is required", "error");
			return;
		}

		const perms = workspacePermissions();
		if (Object.keys(perms).length === 0) {
			toast("At least one workspace permission is required", "error");
			return;
		}

		if (isSaving()) return;
		setIsSaving(true);
		try {
			// Super-admin selections round-trip. Role-grant ceilings aren't
			// editable here until the token-screen rework, so carry the saved
			// ones through untouched rather than dropping them.
			const body: UpdateApiTokenRequest = {
				name,
				superAdminOf: Object.entries(perms)
					.filter(([, permission]) => permission.type === "superAdmin")
					.map(([workspaceId]) => workspaceId),
				grants: info?.grants ?? {},
				tokenNbf: info?.tokenNbf,
				tokenExp: info?.tokenExp,
				allowedIps: info?.allowedIps,
				created: info?.created ?? new Date(),
			};

			const response = await httpRequest<null>(
				`${import.meta.env.VITE_BASE_URL}/api/user/api-token/${params().id}`,
				{
					method: "PATCH",
					body: JSON.stringify(body),
				}
			);

			if (!response.ok) {
				console.error("Failed to update API Token:", response.data.error);
				toast(response.data?.error || "Failed to update API Token", "error");
				return;
			}

			queryClient.setQueryData<GetApiTokenInfoResponse>(apiTokenKeys.detail(params().id), (prev) =>
				prev ? { ...prev, name, superAdminOf: body.superAdminOf, grants: body.grants } : prev
			);
			toast("API Token updated successfully", "success");
		} finally {
			setIsSaving(false);
		}
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
							<InputWithLabel for="token-id" label="ID">
								<Input
									value={apiTokenInfo()?.id || ""}
									disabled={true}
									id="token-id"
									name="token-id"
									placeholder="Token ID"
									type={InputType.Text}
								/>
							</InputWithLabel>

							<form
								onSubmit={(e) => {
									e.preventDefault();
									saveToken();
								}}
								class="w-full"
							>
								<InputWithLabel for="token-name" label="Name">
									<div class="flex gap-2 items-center w-full">
										<Input
											value={tokenName() ?? ""}
											class="flex-1"
											id="token-name"
											name="token-name"
											placeholder="Token Name"
											type={InputType.Text}
											onInput={(e) => setTokenName(e.currentTarget.value)}
										/>
										<Button
											type="submit"
											variant={ButtonVariant.Contained}
											disabled={isSaving() || (tokenName() ?? "").trim() === ""}
										>
											Save
										</Button>
									</div>
								</InputWithLabel>
							</form>
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
										<Button
											variant={ButtonVariant.Contained}
											onClick={saveToken}
											disabled={isSaving() || enabledWorkspaces().size === 0}
										>
											Save Permissions
										</Button>
									</div>

									<For
										each={workspacesQuery.data?.workspaces || []}
										fallback={<div class="text-gray-400">No workspaces available</div>}
									>
										{(ws) => (
											<WorkspacePermissionItem
												workspace={ws}
												isSuperAdmin={userInfo()?.id === ws.superAdminId}
												enabled={enabledWorkspaces().has(ws.id)}
												initialPermission={workspacePermissions()[ws.id]}
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

export const Route = createFileRoute("/_logged-in/profile/api-tokens/$id")({
	component: ApiTokenInfo,
});
