import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createResource, createSignal, For, Suspense } from "solid-js";
import { Button, ButtonVariant, PageContainer, PageContainerBody, PageContainerHead } from "~/components";
import Input, { InputType } from "~/components/input";
import InputLabel from "~/components/input-label";
import { httpRequest } from "~/utils/http-request";
import { useAuthState } from "~/hooks";
import { useUserInfo } from "~/hooks/state-hooks";
import {
	CreateApiTokenRequest,
	CreateApiTokenResponse,
	ListUserWorkspacesResponse,
	WorkspacePermission,
} from "~/bindings";
import { useToast } from "~/components/toast";
import { useNavigate } from "@tanstack/solid-router";
import ApiTokenModal from "./-components/api-token-modal";
import WorkspacePermissionItem from "./-components/workspace-permission-item";

const CreateApiTokens = () => {
	const [authState] = useAuthState();
	const userInfo = useUserInfo();
	const toast = useToast();
	const navigate = useNavigate();

	const [openCopyModal, setOpenCopyModal] = createSignal<boolean>(false);
	const [apiToken, setApiToken] = createSignal<string>("");

	const [workspaces] = createResource(authState, async (auth) => {
		if (!auth || auth.type !== "LoggedIn") {
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

	const [name, setName] = createSignal<string>("");
	const [fromDate, setFromDate] = createSignal<Date | null>(null);
	const [toDate, setToDate] = createSignal<Date | null>(null);

	const [enabledWorkspaces, setEnabledWorkspaces] = createSignal<Set<string>>(new Set());
	const [workspacePermissions, setWorkspacePermissions] = createSignal<{
		[workspaceId: string]: WorkspacePermission;
	}>({});

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

	const hasEnabledWorkspaces = () => enabledWorkspaces().size > 0;

	const onSubmit = async (e: Event) => {
		e.preventDefault();
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("User is not logged in", "error");
			return;
		}

		const perms = workspacePermissions();
		if (Object.keys(perms).length === 0) {
			toast("Please enable at least one workspace and configure permissions", "error");
			return;
		}

		const requestBody: Omit<CreateApiTokenRequest, "created"> = {
			name: name(),
			tokenNbf: fromDate() || undefined,
			tokenExp: toDate() || undefined,
			permissions: perms,
		};

		const response = await httpRequest<CreateApiTokenResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/api-token`,
			{
				method: "POST",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			console.error("Failed to create API token:", response.data.error);
			toast("Failed to create API token", "error");
			return;
		}

		if (response?.data?.token) {
			setOpenCopyModal(true);
			setApiToken(response.data.token);
		}
	};

	return (
		<>
			<Title>New API Token | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Profile",
							url: "/profile",
						},
						{
							label: "API Token",
							url: "/profile/api-tokens",
						},
						{
							label: "Create New API Token",
						},
					]}
					subText="Create API Token"
				/>
				<PageContainerBody class="flex flex-col justify-between gap-8">
					<form onSubmit={onSubmit} class="flex w-full flex-col justify-between gap-8 h-full flex-1">
						<div class="flex flex-col gap-6 items-start w-full">
							<h1 class="text-md">Create API Tokens</h1>

							<div class="flex gap-8 items-center w-full">
								<InputLabel parentClass="flex-2" for="token-name" label="Token Name" />
								<Input
									value={name()}
									onInput={(e) => {
										setName(e.currentTarget.value);
									}}
									class="flex-10"
									name="token-name"
									placeholder="Enter Token Name"
									type={InputType.Text}
								/>
							</div>

							<div class="flex gap-8 items-center w-full">
								<InputLabel
									parentClass="flex-2"
									for="allowed-ips"
									label="Allowed IP(s)"
									comments="By default, all IP addresses will be allowed. Enter Comma Separated Values."
								/>
								<Input
									class="flex-10"
									name="token-name"
									placeholder="Enter Comma Seperated IP(s)"
									type={InputType.Text}
								/>
							</div>

							<div class="flex gap-8 items-center w-full">
								<InputLabel
									parentClass="flex-2"
									label="Token Validity"
									comments="By default, the token will be valid forever from the date created."
								/>

								<div class="flex items-center flex-10 gap-4">
									<InputLabel parentClass="flex-2" for="token-validity-from" label="Valid From" />
									<Input
										class="flex-10"
										value={fromDate() ? (fromDate()?.toISOString().split("T")[0] ?? "") : ""}
										onInput={(e) => {
											setFromDate(e.currentTarget.valueAsDate);
										}}
										name="token-validity"
										placeholder="Enter Token Validity in days"
										type={InputType.Date}
									/>

									<InputLabel parentClass="flex-2 items-center" for="token-validity-to" label="to" />
									<Input
										onInput={(e) => {
											setToDate(e.currentTarget.valueAsDate);
										}}
										value={toDate() ? toDate()!.toISOString().split("T")[0] : ""}
										class="flex-10"
										name="token-validity"
										placeholder="Enter Token Validity in days"
										type={InputType.Date}
									/>
								</div>
							</div>

							<Suspense
								fallback={
									<div class="flex items-center justify-center py-8">
										<div class="text-gray-400">Loading workspaces...</div>
									</div>
								}
							>
								<div class="flex flex-col gap-4 items-start w-full">
									<InputLabel parentClass="flex-2" label="Workspace Permissions" />

									<For
										each={workspaces.latest?.workspaces || []}
										fallback={<div class="text-gray-400">No workspaces available</div>}
									>
										{(ws) => (
											<WorkspacePermissionItem
												workspace={ws}
												isSuperAdmin={userInfo()?.id === ws.superAdminId}
												enabled={enabledWorkspaces().has(ws.id)}
												onToggle={handleWorkspaceToggle}
												onPermissionChange={handlePermissionChange}
											/>
										)}
									</For>

									{!hasEnabledWorkspaces() && (workspaces.latest?.workspaces?.length ?? 0) > 0 && (
										<p class="text-sm text-gray-400">
											Enable at least one workspace to create an API token.
										</p>
									)}
								</div>
							</Suspense>
						</div>

						<div class="flex justify-end">
							<Button type="submit" variant={ButtonVariant.Contained} disabled={!hasEnabledWorkspaces()}>
								Create Token
							</Button>
						</div>
					</form>
					<ApiTokenModal
						isOpen={openCopyModal}
						setIsOpen={setOpenCopyModal}
						token={apiToken}
						onClose={() => navigate({ to: "/profile/api-tokens" })}
					/>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/profile/api-tokens/new")({
	component: CreateApiTokens,
});
