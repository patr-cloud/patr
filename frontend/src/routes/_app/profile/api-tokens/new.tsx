import { createFileRoute } from "@tanstack/solid-router";
import { createMemo, createResource, createSignal, For, Show, Suspense } from "solid-js";
import {
	Button,
	ButtonVariant,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Table,
} from "~/components";
import Input, { InputType } from "~/components/input";
import InputLabel from "~/components/input-label";
import { httpRequest } from "~/utils/http-request";
import { useAuthState } from "~/hooks";
import {
	CreateApiTokenRequest,
	CreateApiTokenResponse,
	ListUserWorkspacesResponse,
	ResourcePermissionType,
	WithId,
	Workspace,
	WorkspacePermission,
} from "~/bindings";
import { useToast } from "~/components/toast";
import { useNavigate } from "@tanstack/solid-router";
import ApiTokenModal from "./-components/api-token-modal";
import PermissionSelector from "../../_workspaced/workspace/roles/-components/permission-selector";
import { parsePermissionName } from "~/utils/func";
import { useFetchPermissions } from "~/hooks/fetch";
import { FiTrash2, FiXCircle } from "solid-icons/fi";

const WorkspacePermissionItem = (props: { workspace: WithId<Workspace> }) => {
	const [selectedPermissionIds, setSelectedPermissionIds] = createSignal<Set<string>>(new Set());
	const [permissionsData, setPermissionsData] = createSignal<{ [key: string]: ResourcePermissionType }>({});
	const [expand, setExpand] = createSignal(false);

	// Fetch all permissions for the workspace to map IDs to names
	const [allPermissions] = useFetchPermissions(props.workspace.id);

	// Create a map of permission ID to permission name
	const permissionIdToName = createMemo(() => {
		const perms = allPermissions()?.permissions;
		if (!perms) return new Map<string, string>();
		return new Map(perms.map((perm) => [perm.id, perm.name]));
	});

	const permissionEntries = createMemo(() => {
		const permissions = permissionsData();
		if (!permissions) return [];
		const nameMap = permissionIdToName();

		// Group permissions by resourceType
		const grouped = new Map<
			string,
			{
				permissionResourceType: string;
				permissionActions: Array<{ permissionId: string; action: string }>;
				permissionType: string;
				resources?: string[];
			}
		>();

		Object.entries(permissions).forEach(([permissionId, permissionData]) => {
			const permissionName = nameMap.get(permissionId) || permissionId;
			const parsed = parsePermissionName(permissionName);

			if (!grouped.has(parsed.resourceType)) {
				grouped.set(parsed.resourceType, {
					permissionResourceType: parsed.resourceType,
					permissionActions: [],
					permissionType: permissionData?.permissionType || "all",
					resources: permissionData?.permissionType ? permissionData.resources : undefined,
				});
			}

			const group = grouped.get(parsed.resourceType)!;
			group.permissionActions.push({
				permissionId,
				action: parsed.action,
			});
		});

		return Array.from(grouped.values());
	});

	return (
		<div class="w-full flex flex-col items-start justify-start">
			<div
				onClick={(e) => {
					e.preventDefault();
					setExpand(!expand());
				}}
				class="flex items-center justify-start gap-4"
			>
				<span
					class="text-[8px] transition-transform duration-200"
					style={{ display: "inline-block", transform: expand() ? "rotate(90deg)" : "rotate(0deg)" }}
				>
					&#9658;
				</span>
				<h1>{props.workspace.name}</h1>
			</div>
			<div class="flex items-center gap-2 w-full">
				{expand() && (
					<div class="flex flex-col items-start gap-4 w-full">
						<div class="flex items-center gap-2 w-full">
							<PermissionSelector
								class="flex-1 w-full"
								workspaceId={props.workspace.id}
								selectedPermissionIds={selectedPermissionIds()}
								onPermissionChange={setSelectedPermissionIds}
								onPermissionsDataChange={(data) => setPermissionsData((prev) => ({ ...prev, ...data }))}
							/>
						</div>

						<Table
							column_grids={["flex-2", "flex-3", "flex-2", "flex-[0.5]"]}
							headings={["Resource Type", "Actions", "Resources", ""]}
							rows={permissionEntries().sort((a, b) =>
								a.permissionResourceType.localeCompare(b.permissionResourceType)
							)}
							renderRow={(perm) => (
								<tr class="table-row">
									<td class="flex-2 flex items-center justify-center">
										<span class="truncate">{perm.permissionResourceType}</span>
									</td>
									<td class="flex-3 flex items-center justify-center">
										<div class="flex flex-wrap gap-1 justify-center">
											<For each={perm.permissionActions}>
												{(actionData) => (
													<span
														onClick={() => {
															const newPermissionsData = { ...permissionsData() };
															delete newPermissionsData[actionData.permissionId];
															setPermissionsData(newPermissionsData);
														}}
														class="text-sm px-2 py-1 bg-secondary-medium rounded cursor-pointer hover:bg-secondary-dark transition-colors flex items-center justify-center gap-1"
													>
														{actionData.action}
														<FiXCircle size={12} class="inline-block" />
													</span>
												)}
											</For>
										</div>
									</td>
									<td class="flex-2 flex items-center justify-center">
										<Show
											when={perm.resources && perm.resources.length > 0}
											fallback={<span class="text-gray-400">All resources</span>}
										>
											<div class="flex flex-col gap-1">
												<For each={perm.resources}>
													{(resource) => <span class="text-sm">{resource}</span>}
												</For>
											</div>
										</Show>
									</td>
									<td
										onClick={() => {
											const newPermissionsData = { ...permissionsData() };
											// Delete all permissions for this resource type
											perm.permissionActions.forEach((actionData) => {
												delete newPermissionsData[actionData.permissionId];
											});
											setPermissionsData(newPermissionsData);
										}}
									>
										<FiTrash2 color="red" />
									</td>
								</tr>
							)}
						/>
					</div>
				)}
			</div>
		</div>
	);
};

export interface WorkspacePermissions {
	[workspaceId: string]: WorkspacePermission;
}

const CreateApiTokens = () => {
	const [authState, _] = useAuthState();
	const toast = useToast();
	const navigate = useNavigate();

	const [openCopyModal, setOpenCopyModal] = createSignal<boolean>(false);
	const [apiToken, setApiToken] = createSignal<string>("");

	const [workspaces] = createResource(authState, async (auth) => {
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
	const [selectedWorkspace, setSelectedWorkspace] = createSignal<string | undefined>(undefined);

	const [workspacePermissions, setWorkspacePermissions] = createSignal<WorkspacePermissions>();

	const selectedWorkspaceInfo = createMemo(() => {
		return workspaces()?.workspaces?.find((ws) => ws.id === selectedWorkspace());
	});

	const onSubmit = async (e: Event) => {
		e.preventDefault();
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("User is not logged in", "error");
			console.error("User is not logged in");
			return;
		}

		console.log("Creating API Token with details:", {
			name: name(),
			fromDate: fromDate(),
			toDate: toDate(),
		});

		// @ts-expect-error
		const requestBody: CreateApiTokenRequest = {
			name: name(),
			tokenNbf: fromDate() || undefined,
			tokenExp: toDate() || undefined,
			permissions: workspacePermissions(),
		};

		const response = await httpRequest<CreateApiTokenResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/api-token`,
			{
				method: "POST",
				body: JSON.stringify(requestBody),
			}
		);

		console.log("API Token created successfully:", response.data);

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
							<div class="flex flex-col gap-2 items-center w-full">
								{/* Workspace Selection Section */}
								<div class="flex gap-8 items-center justify-center w-full">
									<InputLabel parentClass="flex-2" label="Workspace" />
								</div>

								<For
									each={workspaces.latest?.workspaces || []}
									fallback={<div class="text-gray-400">No workspaces available</div>}
								>
									{(ws) => <WorkspacePermissionItem workspace={ws} />}
								</For>
							</div>
						</Suspense>
					</div>

					<div class="flex justify-end">
						<Button type="submit" variant={ButtonVariant.Contained}>
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
	);
};

export const Route = createFileRoute("/_app/profile/api-tokens/new")({
	component: CreateApiTokens,
});
