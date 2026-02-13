import { createMemo, createResource, createSignal, For, Show, Suspense } from "solid-js";
import {
	Button,
	ButtonVariant,
	Input,
	InputDropdown,
	InputLabel,
	InputType,
	Modal,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
} from "~/components";
import { httpRequest } from "~/utils/http-request";
import { useAuthState } from "~/hooks";
import {
	CreateApiTokenRequest,
	CreateApiTokenResponse,
	ListUserWorkspacesResponse,
	WorkspacePermission,
} from "~/bindings";
import { useToast } from "~/components/toast";
import WorkspaceRoles from "~/pages/user/workspace-roles";
import { useNavigate } from "@solidjs/router";
import ApiTokenModal from "./api-token-modal";

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
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.type === "LoggedIn" ? auth.accessToken : ""}`,
				},
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

		// @ts-expect-error we are not using the `created` field for create requests
		const requestBody: CreateApiTokenRequest = {
			name: name(),
			tokenNbf: fromDate() || undefined,
			tokenExp: toDate() || undefined,
			permissions: workspacePermissions(),
		};

		const response = await httpRequest<CreateApiTokenResponse>(`${import.meta.env.VITE_BASE_URL}/api/user/api-token`, {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
				Authorization: `Bearer ${auth.type === "LoggedIn" ? auth.accessToken : ""}`,
			},
			body: JSON.stringify(requestBody),
		});

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
						label: "Settings",
						url: "/profile",
					},
					{
						label: "API Tokens",
						url: "/profile/api-tokens",
					},
					{
						label: "Create API Token",
					},
				]}
				subText="API Tokens can be used to interact with the Patr API programmatically on your behalf."
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

						<div class="flex flex-col gap-2 items-center w-full">
							{/* Workspace Selection Section */}
							<div class="flex gap-8 items-center justify-center w-full">
								<InputLabel parentClass="flex-2" label="Workspace" />
								<InputDropdown
									placeholder="Select Workspace Type"
									class="flex-10"
									onSelect={(val) => {
										console.log(val);
										setSelectedWorkspace(val);
									}}
									value={selectedWorkspace()}
									options={() =>
										workspaces()?.workspaces.map((ws) => ({
											label: ws.name,
											value: ws.id,
										})) || []
									}
								/>
							</div>

							<div class="flex flex-col items-start justify-center gap-4 w-full">
								{/* Member / Super Admin Section */}
								<Show when={selectedWorkspaceInfo()}>
									<div class="flex items-center justify-start gap-8 w-full h-9">
										<InputLabel
											parentClass="flex-2 "
											label={`Permissions for workspace ${selectedWorkspaceInfo()?.name || ""}`}
										/>
										<div class="flex-10 flex items-center justify-start gap-8 w-full h-9">
											<label id="workspace-member-role" class="flex-10 flex items-center gap-4">
												<input
													class=""
													id="workspace-member-role"
													type={InputType.Checkbox}
													checked={workspacePermissions()?.[selectedWorkspace() || ""]?.type === "superAdmin"}
													onChange={(e) => {
														const currentPermissions = workspacePermissions() || {};
														currentPermissions[selectedWorkspace() || ""] = e.currentTarget.checked
															? { type: "superAdmin" }
															: ({ type: "member" } as WorkspacePermission);
														setWorkspacePermissions({ ...currentPermissions });
													}}
												/>
												<p>Super Admin</p>
											</label>
										</div>
									</div>
								</Show>

								<Suspense fallback={<div>Loading...</div>}>
									<Show
										when={
											(workspacePermissions()?.[selectedWorkspace() || ""]?.type === "member" ||
												!workspacePermissions()?.[selectedWorkspace() || ""]) &&
											selectedWorkspaceInfo()
										}
									>
										{(workspace) => (
											<>
												<WorkspaceRoles
													addPermission={(val) => {
														const selectedWS = selectedWorkspace();
														const currentPermissions = workspacePermissions() || {};
														if (!selectedWS) return;

														setWorkspacePermissions({
															...currentPermissions,
															[selectedWS]: val,
														});
													}}
													class="w-full flex-10"
													workspace={() => workspace().id}
												/>

												<For each={Object.entries(workspacePermissions() || {})}>
													{([wsId, perm]) => (
														<div>
															<pre class="text-xs text-gray-400">
																{wsId}: {JSON.stringify(perm)}
															</pre>
														</div>
													)}
												</For>
											</>
										)}
									</Show>
								</Suspense>
							</div>
						</div>
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
					onClose={() => navigate("/profile/api-tokens")}
				/>
			</PageContainerBody>
		</PageContainer>
	);
};
export default CreateApiTokens;
