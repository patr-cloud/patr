import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { useQueryClient } from "@tanstack/solid-query";
import { Title } from "@solidjs/meta";
import { createMemo, createSignal, Show } from "solid-js";
import { FiArrowLeft } from "solid-icons/fi";
import { CreateServiceAccountRequest, CreateServiceAccountResponse } from "~/bindings";
import {
	Alert,
	BindingRows,
	Button,
	ButtonVariant,
	CopyableField,
	Input,
	InputType,
	Label,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	UnsavedChangesGuard,
	useToast,
} from "~/components";
import type { Binding } from "~/components/binding-rows";
import { createFormAction, useGetPermissions, useIsAllowed } from "~/hooks";
import { useAllRolesQuery } from "~/hooks/fetch";
import { roleKeys, serviceAccountKeys } from "~/hooks/query-keys";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { REGISTRY_DOMAIN } from "~/utils/env";
import { httpRequest } from "~/utils/http-request";
import { bindingsToGrants } from "~/utils/scope";
import { RESOURCE_NAME_REGEX, validateRoleDescription } from "~/utils/validation";

/**
 * Create a service account, then show its token once on the same page. The
 * token is only ever held in this page's state: it isn't retrievable again,
 * and it must never reach the URL, history state or the query cache.
 */
const CreateServiceAccount = () => {
	const navigate = useNavigate();
	const toast = useToast();
	const queryClient = useQueryClient();
	const [workspaceId] = useLastWorkspaceId();
	const canCreate = useIsAllowed("serviceAccount", "create");
	const canViewRoles = useIsAllowed("viewRoles", "view");

	// Listing roles needs its own permission. Without it there's no workspace
	// to fetch for, which keeps the query off rather than failing.
	const rolesQuery = useAllRolesQuery(
		() => undefined,
		() => "100",
		() => (canViewRoles() ? (workspaceId() ?? undefined) : undefined)
	);

	const [name, setName] = createSignal("");
	const [nameError, setNameError] = createSignal("");
	const [description, setDescription] = createSignal("");
	const [descriptionError, setDescriptionError] = createSignal("");
	const [bindings, setBindings] = createSignal<Binding[]>([{ subjectId: "", scope: { scopeType: "workspace" } }]);
	const [accessError, setAccessError] = createSignal("");
	const [created, setCreated] = createSignal<{ id: string; name: string; token: string }>();
	const [isLeaving, setIsLeaving] = createSignal(false);
	const [tokenCopied, setTokenCopied] = createSignal(false);

	const createdPermissions = useGetPermissions("serviceAccount", () => created()?.id ?? "");

	const roleOptions = createMemo(() =>
		(rolesQuery.data?.roles ?? []).map((role) => ({ label: role.name, value: role.id }))
	);

	// A resource-scoped grant with nothing selected reaches nothing.
	const hasEmptyResourceScope = createMemo(() =>
		bindings().some(
			(binding) =>
				binding.subjectId && binding.scope.scopeType === "resources" && binding.scope.resources.length === 0
		)
	);

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId: wsId }) => {
		const serviceAccountName = name().trim();
		const serviceAccountDescription = description().trim();

		const nextNameError = !serviceAccountName
			? "Service account name is required."
			: !RESOURCE_NAME_REGEX.test(serviceAccountName)
				? serviceAccountName.length < 2 || serviceAccountName.length > 255
					? "Service account name must be 2 to 255 characters long."
					: "Service account name can only contain letters, numbers, spaces, dots (.), hyphens (-) and underscores (_)."
				: "";
		const grants = bindingsToGrants(bindings(), wsId);
		const nextAccessError = !canViewRoles()
			? "You need permission to view roles to give a service account access."
			: grants.length === 0
				? "Give the service account at least one role."
				: hasEmptyResourceScope()
					? "A role scoped to specific resources needs at least one resource."
					: "";
		const nextDescriptionError = validateRoleDescription(serviceAccountDescription) ?? "";
		setNameError(nextNameError);
		setDescriptionError(nextDescriptionError);
		setAccessError(nextAccessError);
		if (nextNameError || nextDescriptionError || nextAccessError) return;

		const requestBody: CreateServiceAccountRequest = {
			name: serviceAccountName,
			description: serviceAccountDescription || null,
			roleBindings: grants,
		};
		const response = await httpRequest<CreateServiceAccountResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/service-account`,
			{ method: "POST", body: JSON.stringify(requestBody) }
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
				toast("Failed to create the service account. Please try again.", "error");
			}
			return;
		}

		queryClient.invalidateQueries({ queryKey: serviceAccountKeys.all(wsId) });
		queryClient.invalidateQueries({ queryKey: roleKeys.all(wsId) });
		setCreated({ id: response.data.id, name: serviceAccountName, token: response.data.token });
	});

	const goToServiceAccount = () => {
		const account = created();
		if (!account) return;
		setIsLeaving(true);
		if (createdPermissions().view) {
			navigate({ to: "/workspace/service-accounts/$id", params: { id: account.id } });
		} else {
			navigate({ to: "/workspace/service-accounts" });
		}
	};

	return (
		<>
			<Title>New Service Account | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{ label: "Workspace Settings", url: "/workspace" },
						{ label: "Service Accounts", url: "/workspace/service-accounts" },
						{ label: "New" },
					]}
					subText="Give CI pipelines and other automation their own token, with only the roles they need."
				/>
				<PageContainerBody>
					<Show
						when={canCreate()}
						fallback={
							<div class="p-6 bg-secondary-light text-error rounded-xs text-sm">
								You don't have permission to create service accounts in this workspace.
							</div>
						}
					>
						<Show
							when={created()}
							fallback={
								<form noValidate class="flex flex-col gap-6 max-w-5xl" onSubmit={onSubmit}>
									<div class="flex flex-col gap-2">
										<Label for="service-account-name" label="Name" />
										<Input
											id="service-account-name"
											name="service-account-name"
											type={InputType.Text}
											placeholder="e.g. github-actions"
											value={name()}
											onInput={(e) => {
												setName(e.currentTarget.value);
												setNameError("");
											}}
										/>
										<Show when={nameError()}>
											<Alert message={nameError()} type="error" />
										</Show>
									</div>

									<div class="flex flex-col gap-2">
										<Label for="service-account-description" label="Description" />
										<Input
											id="service-account-description"
											name="service-account-description"
											type={InputType.Text}
											placeholder="What uses this service account (optional)"
											maxLength={500}
											value={description()}
											onInput={(e) => {
												setDescription(e.currentTarget.value);
												setDescriptionError(
													validateRoleDescription(e.currentTarget.value) ?? ""
												);
											}}
										/>
										<Show when={descriptionError()}>
											<Alert message={descriptionError()} type="error" />
										</Show>
									</div>

									<div class="flex flex-col gap-2">
										<Label label="Access" />
										<Show
											when={canViewRoles()}
											fallback={
												<p class="text-grey text-sm">
													You need permission to view roles to give a service account access.
													Ask a workspace admin for a role that includes View Roles.
												</p>
											}
										>
											<p class="text-grey text-xs">
												Each row grants one role, either across the whole workspace or on the
												resources you pick. The token can do what these roles allow, and nothing
												more.
											</p>
											<BindingRows
												workspaceId={workspaceId()!}
												bindings={bindings()}
												onChange={(next) => {
													setBindings(next);
													setAccessError("");
												}}
												subjectOptions={roleOptions()}
												subjectPlaceholder="Select a role"
												scopeRoleId={(roleId) => roleId}
												addLabel="Add role"
												emptyText="No roles yet — add one below."
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
										</Show>
										<Show when={accessError()}>
											<Alert message={accessError()} type="error" />
										</Show>
									</div>

									<div class="flex items-center gap-2">
										<Button
											type="submit"
											variant={ButtonVariant.Contained}
											disabled={isLoading() || !canViewRoles()}
											loading={isLoading}
											loadingContent={() => <span>Creating...</span>}
										>
											Create Service Account
										</Button>
										<Button
											variant={ButtonVariant.Outlined}
											class="flex items-center gap-2"
											onClick={() => navigate({ to: "/workspace/service-accounts" })}
										>
											<FiArrowLeft size={14} />
											Cancel
										</Button>
									</div>
								</form>
							}
						>
							{(account) => (
								<div class="flex flex-col gap-6 w-full max-w-5xl">
									<Alert type="warning" message="Copy this token now. It won't be shown again." />
									<p class="text-grey text-sm">
										<span class="text-white">{account().name}</span>&nbsp;is created. Its token can
										do what its roles allow, and nothing more.
									</p>

									<div>
										<p class="text-gray-300 text-sm mb-2">Service account token</p>
										<CopyableField
											value={account().token}
											innerClass="font-mono"
											onCopy={() => setTokenCopied(true)}
										/>
									</div>

									<div>
										<p class="text-gray-300 text-sm mb-2">
											{REGISTRY_DOMAIN
												? "Use it as a bearer token for the Patr API, or as the password for:"
												: "Use it as a bearer token for the Patr API."}
										</p>
										{REGISTRY_DOMAIN && (
											<CopyableField
												value={`docker login ${REGISTRY_DOMAIN} -u patr`}
												innerClass="font-mono"
											/>
										)}
									</div>

									<div class="w-full flex justify-end">
										<Button variant={ButtonVariant.Contained} onClick={goToServiceAccount}>
											Go to Service Account
										</Button>
									</div>
								</div>
							)}
						</Show>
					</Show>
					<UnsavedChangesGuard
						when={() => !!created() && !isLeaving() && !tokenCopied()}
						title="Leave without the token?"
						message="This service account's token won't be shown again. If you haven't copied it, you'll need to regenerate it from the service account's page."
					/>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/workspace/service-accounts/new")({
	component: CreateServiceAccount,
});
