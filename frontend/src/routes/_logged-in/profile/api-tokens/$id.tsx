import { createFileRoute } from "@tanstack/solid-router";
import { useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, createSignal, For, Show, Suspense } from "solid-js";
import { GetApiTokenInfoResponse, UpdateApiTokenRequest } from "~/bindings";
import { PermissionGrant } from "~/bindings/PermissionGrant";
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
import TokenGrantsItem from "./-components/token-grants-item";

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

	// Ceiling editing state: which workspaces the token reaches, and per
	// workspace either super-admin or a set of role grants.
	const [enabledWorkspaces, setEnabledWorkspaces] = createSignal<Set<string>>(new Set());
	const [superAdminWorkspaces, setSuperAdminWorkspaces] = createSignal<Set<string>>(new Set());
	const [workspaceGrants, setWorkspaceGrants] = createSignal<{ [workspaceId: string]: PermissionGrant[] }>({});
	const [initialized, setInitialized] = createSignal(false);

	// Initialize from fetched token info. Any successful query counts as
	// initialised — even with an empty ceiling the section should render so
	// the user can tick a workspace and author grants from scratch.
	createEffect(() => {
		const info = apiTokenInfo();
		if (!info || initialized()) return;

		const enabled = new Set<string>();
		const superAdmin = new Set<string>();
		const grants: { [key: string]: PermissionGrant[] } = {};

		for (const wsId of info.superAdminOf ?? []) {
			enabled.add(wsId);
			superAdmin.add(wsId);
		}
		for (const [wsId, roleGrants] of Object.entries(info.grants ?? {})) {
			enabled.add(wsId);
			grants[wsId] = roleGrants;
		}

		setEnabledWorkspaces(enabled);
		setSuperAdminWorkspaces(superAdmin);
		setWorkspaceGrants(grants);
		setInitialized(true);
	});

	const handleWorkspaceToggle = (workspaceId: string, enabled: boolean) => {
		const newEnabled = new Set(enabledWorkspaces());
		if (enabled) {
			newEnabled.add(workspaceId);
		} else {
			newEnabled.delete(workspaceId);
			const newSuperAdmin = new Set(superAdminWorkspaces());
			newSuperAdmin.delete(workspaceId);
			setSuperAdminWorkspaces(newSuperAdmin);
			const newGrants = { ...workspaceGrants() };
			delete newGrants[workspaceId];
			setWorkspaceGrants(newGrants);
		}
		setEnabledWorkspaces(newEnabled);
	};

	const handleSuperAdminChange = (workspaceId: string, superAdmin: boolean) => {
		const next = new Set(superAdminWorkspaces());
		if (superAdmin) next.add(workspaceId);
		else next.delete(workspaceId);
		setSuperAdminWorkspaces(next);
	};

	const handleGrantsChange = (workspaceId: string, grants: PermissionGrant[]) => {
		setWorkspaceGrants((prev) => ({ ...prev, [workspaceId]: grants }));
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

		if (enabledWorkspaces().size === 0) {
			toast("At least one workspace permission is required", "error");
			return;
		}

		// Every enabled workspace must contribute something to the ceiling —
		// super-admin, or at least one role grant.
		const superAdminOf = [...enabledWorkspaces()].filter((wsId) => superAdminWorkspaces().has(wsId));
		const grants = Object.fromEntries(
			[...enabledWorkspaces()]
				.filter((wsId) => !superAdminWorkspaces().has(wsId))
				.map((wsId) => [wsId, workspaceGrants()[wsId] ?? []])
		);
		if (Object.values(grants).some((roleGrants) => roleGrants.length === 0)) {
			toast("Every enabled workspace needs super admin or at least one role", "error");
			return;
		}

		if (isSaving()) return;
		setIsSaving(true);
		try {
			const body: UpdateApiTokenRequest = {
				name,
				superAdminOf,
				grants,
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

									<p class="text-grey text-sm">
										<span class="text-white">This token can never do more than you can.</span>{" "}
										Whatever you select here is checked against your own permissions each time the
										token is used — if you lose access to something, so does the token.
									</p>

									<For
										each={workspacesQuery.data?.workspaces || []}
										fallback={<div class="text-gray-400">No workspaces available</div>}
									>
										{(ws) => (
											<TokenGrantsItem
												workspace={ws}
												isSuperAdmin={userInfo()?.id === ws.superAdminId}
												enabled={enabledWorkspaces().has(ws.id)}
												superAdmin={superAdminWorkspaces().has(ws.id)}
												grants={workspaceGrants()[ws.id] ?? []}
												onToggle={handleWorkspaceToggle}
												onSuperAdminChange={handleSuperAdminChange}
												onGrantsChange={handleGrantsChange}
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
