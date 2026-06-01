import { createFileRoute } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, For, Suspense } from "solid-js";
import {
	Button,
	ButtonVariant,
	ChipInput,
	Input,
	InputType,
	InputLabel,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import { httpRequest } from "~/utils/http-request";
import { useAuthState } from "~/hooks";
import { useUserInfo } from "~/hooks/state-hooks";
import { useWorkspacesQuery } from "~/hooks/fetch";
import { CreateApiTokenRequest, CreateApiTokenResponse, WorkspacePermission } from "~/bindings";
import { useNavigate } from "@tanstack/solid-router";
import ApiTokenModal from "./-components/api-token-modal";
import WorkspacePermissionItem from "./-components/workspace-permission-item";
import { validateApiTokenName, validateIp } from "~/utils/validation";

const CreateApiTokens = () => {
	const [authState] = useAuthState();
	const userInfo = useUserInfo();
	const toast = useToast();
	const navigate = useNavigate();

	const [openCopyModal, setOpenCopyModal] = createSignal<boolean>(false);
	const [apiToken, setApiToken] = createSignal<string>("");

	const workspacesQuery = useWorkspacesQuery();

	const [name, setName] = createSignal<string>("");
	const [nameError, setNameError] = createSignal("");
	const [allowedIps, setAllowedIps] = createSignal<string[]>([]);
	const [fromDate, setFromDate] = createSignal<Date | null>(null);
	const [toDate, setToDate] = createSignal<Date | null>(null);
	const [dateError, setDateError] = createSignal("");

	const checkName = (): boolean => {
		const r = validateApiTokenName(name());
		setNameError(r.valid ? "" : (r.error ?? ""));
		return r.valid;
	};

	const checkDates = (): boolean => {
		const f = fromDate();
		const t = toDate();
		const now = new Date();
		if (t && t <= now) {
			setDateError("Valid-to must be in the future.");
			return false;
		}
		if (f && t && f > t) {
			setDateError("Valid-from must be on or before Valid-to.");
			return false;
		}
		setDateError("");
		return true;
	};

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

		if (!checkName() || !checkDates()) return;

		const perms = workspacePermissions();
		if (Object.keys(perms).length === 0) {
			toast("Please enable at least one workspace and configure permissions", "error");
			return;
		}

		const requestBody: Omit<CreateApiTokenRequest, "created"> = {
			name: name(),
			tokenNbf: fromDate() || undefined,
			tokenExp: toDate() || undefined,
			allowedIps: allowedIps().length > 0 ? allowedIps() : undefined,
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
									id="token-name"
									value={name()}
									onInput={(e) => {
										setName(e.currentTarget.value);
										setNameError("");
									}}
									onBlur={() => checkName()}
									error={nameError}
									class="flex-10"
									name="token-name"
									placeholder="Enter Token Name"
									type={InputType.Text}
								/>
							</div>

							<div class="flex gap-8 items-start w-full">
								<InputLabel
									parentClass="flex-2 pt-2.5"
									label="Allowed IP(s)"
									comments="Leave empty to allow all IPs."
								/>
								<ChipInput
									class="flex-10"
									values={allowedIps}
									onChange={setAllowedIps}
									validate={validateIp}
									placeholder="Type an IP address and press Enter, Space, or Comma"
								/>
							</div>

							<div class="flex gap-8 items-center w-full">
								<InputLabel
									parentClass="flex-2"
									label="Token Validity"
									comments="By default, the token will be valid forever from the date created."
								/>

								<div class="flex flex-col flex-10 gap-1">
									<div class="flex items-center gap-4">
										<InputLabel parentClass="flex-2" for="token-validity-from" label="Valid From" />
										<Input
											id="token-validity-from"
											class="flex-10"
											value={fromDate() ? (fromDate()?.toISOString().split("T")[0] ?? "") : ""}
											onInput={(e) => {
												setFromDate(e.currentTarget.valueAsDate);
												setDateError("");
											}}
											onBlur={() => checkDates()}
											name="token-validity"
											placeholder="Enter Token Validity in days"
											type={InputType.Date}
										/>

										<InputLabel parentClass="flex-2 items-center" for="token-validity-to" label="to" />
										<Input
											id="token-validity-to"
											onInput={(e) => {
												setToDate(e.currentTarget.valueAsDate);
												setDateError("");
											}}
											onBlur={() => checkDates()}
											value={toDate() ? toDate()!.toISOString().split("T")[0] : ""}
											class="flex-10"
											name="token-validity"
											placeholder="Enter Token Validity in days"
											type={InputType.Date}
											error={dateError}
										/>
									</div>
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
										each={workspacesQuery.data?.workspaces || []}
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

									{!hasEnabledWorkspaces() && (workspacesQuery.data?.workspaces?.length ?? 0) > 0 && (
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
