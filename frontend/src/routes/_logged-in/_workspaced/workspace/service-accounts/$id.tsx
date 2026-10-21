import { createFileRoute, notFound, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { useQueryClient } from "@tanstack/solid-query";
import { createEffect, createMemo, createSignal, ErrorBoundary, For, Show } from "solid-js";
import {
	DeleteServiceAccountResponse,
	RegenerateServiceAccountTokenResponse,
	RoleBindingGrant,
	UpdateServiceAccountRequest,
	UpdateServiceAccountResponse,
} from "~/bindings";
import {
	AccessRow,
	Alert,
	BindingRows,
	Button,
	ButtonVariant,
	CopyableField,
	CopyableFieldVariant,
	DeleteModal,
	Input,
	InputType,
	Label,
	Link,
	LoadingSpinner,
	Modal,
	ModalContainer,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	UnsavedChangesGuard,
	useToast,
} from "~/components";
import type { Binding } from "~/components/binding-rows";
import { createAuthenticatedAction, createFormAction, useGetPermissions, useIsAllowed, useIsMounted } from "~/hooks";
import { useAllRolesQuery, useServiceAccountQuery } from "~/hooks/fetch";
import { roleKeys, serviceAccountKeys } from "~/hooks/query-keys";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { Color } from "~/utils/color";
import { httpRequest } from "~/utils/http-request";
import { bindingsToGrants, grantsToBindings } from "~/utils/scope";
import { RESOURCE_NAME_REGEX, validateRoleDescription } from "~/utils/validation";
import RegenerateModal from "~/routes/_logged-in/profile/api-tokens/-components/regenerate-modal";

const ServiceAccountForm = (props: { serviceAccountId: string }) => {
	const toast = useToast();
	const queryClient = useQueryClient();
	const [workspaceId] = useLastWorkspaceId();
	const isMounted = useIsMounted();

	const serviceAccountQuery = useServiceAccountQuery(() => props.serviceAccountId);
	const permissions = useGetPermissions("serviceAccount", () => props.serviceAccountId);
	const canViewRoles = useIsAllowed("viewRoles", "view");

	// Listing roles needs its own permission. Without it there's no workspace
	// to fetch for, which keeps the query off rather than failing.
	const rolesQuery = useAllRolesQuery(
		() => undefined,
		() => "100",
		() => (canViewRoles() ? (workspaceId() ?? undefined) : undefined)
	);

	// Checked before reading the data, so a cold roles query doesn't suspend the page.
	const roles = createMemo(() => (rolesQuery.isSuccess ? rolesQuery.data.roles : []));
	const roleNames = createMemo(() => new Map(roles().map((role) => [role.id, role.name])));
	const roleOptions = createMemo(() => roles().map((role) => ({ label: role.name, value: role.id })));
	const rolesSettled = () => isMounted() && (!canViewRoles() || !rolesQuery.isPending);

	const [name, setName] = createSignal<string>();
	const [nameError, setNameError] = createSignal("");
	const [description, setDescription] = createSignal<string>();
	const [descriptionError, setDescriptionError] = createSignal("");
	const [accessError, setAccessError] = createSignal("");
	// The bindings being edited, and the roles that could be edited when they
	// were seeded. Grants on any other role aren't shown in the editor, and go
	// back on save exactly as they were fetched.
	const [bindings, setBindings] = createSignal<Binding[]>();
	const [editableRoleIds, setEditableRoleIds] = createSignal(new Set<string>());

	// Seed the form once the service account loads. Only once: the query
	// refetches whenever the window regains focus, and reseeding then would
	// wipe out an edit in progress.
	createEffect(() => {
		const serviceAccount = serviceAccountQuery.data;
		if (serviceAccount && name() === undefined) {
			setName(serviceAccount.name);
			setDescription(serviceAccount.description ?? "");
		}
	});

	// The bindings also wait for the roles, since only a grant on a role that
	// loaded can be picked in the editor.
	createEffect(() => {
		const serviceAccount = serviceAccountQuery.data;
		const wsId = workspaceId();
		if (!serviceAccount || !wsId || !permissions().edit || !rolesQuery.isSuccess || bindings() !== undefined) {
			return;
		}
		const known = new Set(rolesQuery.data.roles.map((role) => role.id));
		setEditableRoleIds(known);
		setBindings(
			grantsToBindings(
				serviceAccount.roleBindings.filter((grant) => known.has(grant.roleId)),
				wsId
			)
		);
	});

	const fetchedGrants = () => serviceAccountQuery.data?.roleBindings ?? [];

	const lockedBindings = createMemo(() =>
		grantsToBindings(
			fetchedGrants().filter((grant) => !editableRoleIds().has(grant.roleId)),
			workspaceId() ?? ""
		)
	);

	/**
	 * What saving sends: every fetched grant as it was, except on the roles
	 * whose binding the user changed.
	 */
	const grantsToSave = createMemo((): RoleBindingGrant[] => {
		const edited = bindings();
		const wsId = workspaceId();
		if (!edited || !wsId) return fetchedGrants();

		const before = new Map(grantsToBindings(fetchedGrants(), wsId).map((binding) => [binding.subjectId, binding]));
		return [
			...fetchedGrants().filter((grant) => !editableRoleIds().has(grant.roleId)),
			...edited.flatMap((binding) =>
				JSON.stringify(before.get(binding.subjectId)?.scope) === JSON.stringify(binding.scope)
					? fetchedGrants().filter((grant) => grant.roleId === binding.subjectId)
					: bindingsToGrants([binding], wsId)
			),
		];
	});

	const isDirty = createMemo(() => {
		const serviceAccount = serviceAccountQuery.data;
		if (!serviceAccount || name() === undefined) return false;

		const key = (grant: RoleBindingGrant) => `${grant.roleId}:${grant.resourceId}`;
		const saved = new Set(serviceAccount.roleBindings.map(key));
		const next = new Set(grantsToSave().map(key));
		return (
			(name() ?? "").trim() !== serviceAccount.name ||
			(description() ?? "").trim() !== (serviceAccount.description ?? "") ||
			saved.size !== next.size ||
			[...next].some((grant) => !saved.has(grant))
		);
	});

	// A resource-scoped grant with nothing selected reaches nothing.
	const hasEmptyResourceScope = createMemo(() =>
		(bindings() ?? []).some(
			(binding) =>
				binding.subjectId && binding.scope.scopeType === "resources" && binding.scope.resources.length === 0
		)
	);

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId: wsId }) => {
		const serviceAccountName = (name() ?? "").trim();
		const serviceAccountDescription = (description() ?? "").trim();

		const nextNameError = !serviceAccountName
			? "Service account name is required."
			: !RESOURCE_NAME_REGEX.test(serviceAccountName)
				? serviceAccountName.length < 2 || serviceAccountName.length > 255
					? "Service account name must be 2 to 255 characters long."
					: "Service account name can only contain letters, numbers, spaces, dots (.), hyphens (-) and underscores (_)."
				: "";
		const nextDescriptionError = validateRoleDescription(serviceAccountDescription) ?? "";
		const nextAccessError = hasEmptyResourceScope()
			? "A role scoped to specific resources needs at least one resource."
			: "";
		setNameError(nextNameError);
		setDescriptionError(nextDescriptionError);
		setAccessError(nextAccessError);
		if (nextNameError || nextDescriptionError || nextAccessError) return;

		const requestBody: UpdateServiceAccountRequest = {
			name: serviceAccountName,
			description: serviceAccountDescription || null,
			roleBindings: grantsToSave(),
		};
		const response = await httpRequest<UpdateServiceAccountResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/service-account/${props.serviceAccountId}`,
			{ method: "PATCH", body: JSON.stringify(requestBody) }
		);

		if (!response.ok) {
			const err = response.data.error;
			if (err === "resourceAlreadyExists") {
				setNameError(`A service account named "${serviceAccountName}" already exists`);
			} else if (err === "roleDoesNotExist") {
				setAccessError("One of these roles was deleted. Remove it, or pick another role, and try again.");
			} else if (err === "resourceDoesNotExist") {
				setAccessError(
					"One of the resources these roles are scoped to was deleted. Remove it from the scope and try again."
				);
			} else {
				toast("Failed to update the service account. Please try again.", "error");
			}
			return;
		}

		setName(serviceAccountName);
		setDescription(serviceAccountDescription);
		await queryClient.invalidateQueries({ queryKey: serviceAccountKeys.all(wsId) });
		queryClient.invalidateQueries({ queryKey: roleKeys.all(wsId) });
		toast("Service account updated successfully", "success");
	});

	return (
		<form noValidate onSubmit={onSubmit} class="flex flex-col gap-6 w-full">
			<div class="flex flex-col gap-4 w-full">
				<div class="flex gap-8 items-start w-full">
					<Label parentClass="flex-2 pt-2.5" label="ID" />
					<div class="flex-10 min-w-0">
						<CopyableField
							value={props.serviceAccountId}
							variant={CopyableFieldVariant.Input}
							buttonPosition="start"
							class="w-full"
						/>
					</div>
				</div>

				<div class="flex gap-8 items-start w-full">
					<Label parentClass="flex-2 pt-2.5" for="service-account-name" label="Name" />
					<div class="flex-10 flex flex-col gap-1">
						<Input
							id="service-account-name"
							name="service-account-name"
							class="w-full"
							type={InputType.Text}
							value={name() ?? ""}
							disabled={!permissions().edit}
							onInput={(e) => {
								setName(e.currentTarget.value);
								setNameError("");
							}}
						/>
						<Show when={nameError()}>
							<Alert message={nameError()} type="error" />
						</Show>
					</div>
				</div>

				<div class="flex gap-8 items-start w-full">
					<Label parentClass="flex-2 pt-2.5" for="service-account-description" label="Description" />
					<div class="flex-10 flex flex-col gap-1">
						<Input
							id="service-account-description"
							name="service-account-description"
							class="w-full"
							type={InputType.Text}
							placeholder={permissions().edit ? "What uses this service account (optional)" : ""}
							maxLength={500}
							value={description() ?? ""}
							disabled={!permissions().edit}
							onInput={(e) => {
								setDescription(e.currentTarget.value);
								setDescriptionError(validateRoleDescription(e.currentTarget.value) ?? "");
							}}
						/>
						<Show when={descriptionError()}>
							<Alert message={descriptionError()} type="error" />
						</Show>
					</div>
				</div>

				<div class="flex gap-8 items-start w-full">
					<Label parentClass="flex-2 pt-2.5" label="Access" />
					<div class="flex-10 min-w-0 flex flex-col gap-3 pt-2.5">
						<Show
							when={rolesSettled()}
							fallback={
								<div class="flex items-center gap-2 text-grey">
									<LoadingSpinner size={16} />
									<span class="text-sm">Loading roles...</span>
								</div>
							}
						>
							<Show
								when={bindings()}
								fallback={
									<>
										<Show when={permissions().edit && !canViewRoles()}>
											<p class="text-grey text-xs">
												You need permission to view roles to change this service account's
												roles. Ask a workspace admin for a role that includes View Roles.
											</p>
										</Show>
										<Show
											when={!permissions().edit && !canViewRoles() && fetchedGrants().length > 0}
										>
											<p class="text-grey text-xs">
												Roles are shown by ID because you don't have permission to view roles in
												this workspace.
											</p>
										</Show>
										<Show
											when={fetchedGrants().length > 0}
											fallback={<p class="text-grey text-sm italic">No roles assigned.</p>}
										>
											<ul class="flex flex-col gap-2">
												<For each={grantsToBindings(fetchedGrants(), workspaceId() ?? "")}>
													{(binding) => (
														<AccessRow
															label={
																roleNames().get(binding.subjectId) ?? binding.subjectId
															}
															scope={binding.scope}
														/>
													)}
												</For>
											</ul>
										</Show>
									</>
								}
							>
								{(edited) => (
									<>
										<p class="text-grey text-xs">
											Each row grants one role, either across the whole workspace or on the
											resources you pick. The token can do what these roles allow, and nothing
											more.
										</p>
										<Show when={lockedBindings().length > 0}>
											<p class="text-grey text-xs">
												These roles couldn't be loaded, so they're kept as they are.
											</p>
											<ul class="flex flex-col gap-2">
												<For each={lockedBindings()}>
													{(binding) => (
														<AccessRow label={binding.subjectId} scope={binding.scope} />
													)}
												</For>
											</ul>
										</Show>
										<BindingRows
											workspaceId={workspaceId()!}
											bindings={edited()}
											onChange={(next) => {
												setBindings(next);
												setAccessError("");
											}}
											subjectOptions={roleOptions()}
											subjectPlaceholder="Select a role"
											scopeRoleId={(roleId) => roleId}
											addLabel="Add role"
											emptyText="No roles assigned."
											footer={() => (
												<a
													href="/workspace/roles/new"
													target="_blank"
													rel="noopener noreferrer"
													class="text-primary text-xs hover:underline self-start"
												>
													or create a new role &rarr;
												</a>
											)}
										/>
									</>
								)}
							</Show>
						</Show>
						<Show when={accessError()}>
							<Alert message={accessError()} type="error" />
						</Show>
					</div>
				</div>
			</div>

			<Show when={permissions().edit}>
				<div class="w-full flex justify-end">
					<Button
						variant={ButtonVariant.Contained}
						type="submit"
						disabled={!isDirty() || isLoading()}
						loading={isLoading()}
						loadingContent={() => <span>Saving...</span>}
					>
						Save Changes
					</Button>
				</div>
			</Show>

			<UnsavedChangesGuard
				when={isDirty}
				message="You have unsaved changes to this service account. If you leave now, they'll be lost."
			/>
		</form>
	);
};

const ServiceAccountDetail = () => {
	const params = Route.useParams();
	const navigate = useNavigate();
	const toast = useToast();
	const queryClient = useQueryClient();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);
	const [isRegenerateModalOpen, setIsRegenerateModalOpen] = createSignal(false);
	const [newToken, setNewToken] = createSignal("");

	const permissions = useGetPermissions("serviceAccount", () => params().id);
	const serviceAccountQuery = useServiceAccountQuery(() => params().id);

	const { execute: deleteServiceAccount, isLoading: isDeleting } = createAuthenticatedAction(
		async ({ workspaceId: wsId }) => {
			const response = await httpRequest<DeleteServiceAccountResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/service-account/${params().id}`,
				{ method: "DELETE" }
			);

			if (!response.ok) {
				toast("Failed to delete the service account", "error");
				return;
			}

			setIsDeleteModalOpen(false);
			toast("Service account deleted successfully", "success");
			// Leave first, so the invalidation below doesn't refetch the deleted account.
			await navigate({ to: "/workspace/service-accounts" });
			queryClient.invalidateQueries({ queryKey: serviceAccountKeys.all(wsId) });
			queryClient.invalidateQueries({ queryKey: roleKeys.all(wsId) });
		}
	);

	const { execute: regenerateToken, isLoading: isRegenerating } = createAuthenticatedAction(
		async ({ workspaceId: wsId }) => {
			const response = await httpRequest<RegenerateServiceAccountTokenResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/service-account/${params().id}/token`,
				{ method: "POST" }
			);

			if (!response.ok) {
				toast("Failed to regenerate the service account's token", "error");
				return;
			}

			setIsRegenerateModalOpen(false);
			setNewToken(response.data.token);
			queryClient.invalidateQueries({ queryKey: serviceAccountKeys.detail(wsId, params().id) });
		}
	);

	return (
		<>
			<Title>Service Account Details | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{ label: "Workspace Settings", url: "/workspace" },
						{ label: "Service Accounts", url: "/workspace/service-accounts" },
						{ label: serviceAccountQuery.data?.name || "Loading..." },
					]}
					subText={
						permissions().edit
							? permissions().regenerateToken
								? "Edit this service account's details and roles, or regenerate its token."
								: "Edit this service account's details and roles."
							: permissions().regenerateToken
								? "View this service account's details, or regenerate its token."
								: "View this service account's details and roles."
					}
					actions={() => (
						<div class="flex items-center gap-sm">
							<Show when={serviceAccountQuery.data && permissions().regenerateToken}>
								<RegenerateModal
									title="Regenerate Service Account Token"
									message="The current token stops working immediately. Anything using it, such as a CI pipeline, fails until you give it the new one."
									resourceName={serviceAccountQuery.data?.name || ""}
									isOpen={isRegenerateModalOpen}
									setIsOpen={setIsRegenerateModalOpen}
									isLoading={isRegenerating()}
									onClickRegenerate={(e) => {
										e.preventDefault();
										regenerateToken();
									}}
									renderTrigger={(open) => (
										<Button
											onClick={() => open(true)}
											variant={ButtonVariant.Outlined}
											color={Color.Error}
										>
											Regenerate Token
										</Button>
									)}
								/>
							</Show>
							<Show when={serviceAccountQuery.data && permissions().delete}>
								<DeleteModal
									isLoading={isDeleting()}
									title="Do You Really Want to Delete This Service Account?"
									message="Its token stops working immediately and its role bindings are removed. This can't be undone."
									resourceName={serviceAccountQuery.data?.name || ""}
									isOpen={isDeleteModalOpen}
									setIsOpen={setIsDeleteModalOpen}
									onClickDelete={(e) => {
										e.preventDefault();
										deleteServiceAccount();
									}}
									renderTrigger={(open) => (
										<Button
											class="h-10"
											onClick={() => open?.(true)}
											variant={ButtonVariant.Outlined}
											color={Color.Error}
										>
											Delete
										</Button>
									)}
								/>
							</Show>
						</div>
					)}
				/>
				<PageContainerBody class="flex flex-col gap-6">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading service account: {err.message}</p>
								<button class="text-primary hover:underline text-sm" onClick={reset}>
									Retry
								</button>
							</div>
						)}
					>
						<Show
							when={!serviceAccountQuery.isPending}
							fallback={
								<div class="flex items-center justify-center gap-2 py-16 text-grey">
									<LoadingSpinner size={20} />
									<span class="text-sm">Loading service account...</span>
								</div>
							}
						>
							<Show
								when={serviceAccountQuery.data}
								fallback={
									<div class="flex flex-col items-center justify-center gap-4 py-16">
										<p class="text-error text-sm">
											This service account doesn't exist, or you don't have permission to view it.
										</p>
										<Link href="/workspace/service-accounts" external={false}>
											Back to Service Accounts
										</Link>
									</div>
								}
							>
								<ServiceAccountForm serviceAccountId={params().id} />
							</Show>
						</Show>
					</ErrorBoundary>
				</PageContainerBody>

				<Modal
					isOpen={() => newToken() !== ""}
					renderTrigger={() => <></>}
					renderModalContent={() => (
						<ModalContainer closeFn={() => setNewToken("")} class="max-w-200">
							<h2 class="text-md mb-4 text-primary">New Service Account Token</h2>
							<p class="mb-3 text-sm text-white">
								Copy this token now. You won't be able to see it again.
							</p>
							<CopyableField value={newToken()} innerClass="font-mono" />
						</ModalContainer>
					)}
				/>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/workspace/service-accounts/$id")({
	// The id goes into an API path, so anything but a bare UUID is a 404
	// rather than a request to some other endpoint.
	beforeLoad: ({ params }) => {
		if (!/^[0-9a-f]{32}$/i.test(params.id)) {
			throw notFound();
		}
	},
	component: ServiceAccountDetail,
});
