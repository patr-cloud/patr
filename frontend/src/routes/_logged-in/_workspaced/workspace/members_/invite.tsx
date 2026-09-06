import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, createSignal, Show } from "solid-js";
import { FiArrowLeft, FiSend } from "solid-icons/fi";
import {
	BindingRows,
	Button,
	ButtonVariant,
	Input,
	InputType,
	Label,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import type { Binding } from "~/components/binding-rows";
import { createFormAction, useIsAllowed } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { scopeResources } from "~/utils/scope";
import { useAllRolesQuery, useWorkspaceInfoQuery } from "~/hooks/fetch";
import { InviteUserToWorkspaceRequest } from "~/bindings/InviteUserToWorkspaceRequest";
import { InviteUserToWorkspaceResponse } from "~/bindings/InviteUserToWorkspaceResponse";
import { RoleBindingGrant } from "~/bindings/RoleBindingGrant";
import { httpRequest } from "~/utils/http-request";
import { useQueryClient } from "@tanstack/solid-query";
import { inviteKeys } from "~/hooks/query-keys";

/**
 * Invite someone to the workspace, choosing what they get on acceptance. The
 * bindings authored here are stored on the invite as-is and become the
 * invitee's role bindings the moment they accept — including their scopes.
 */
const InviteMember = () => {
	const navigate = useNavigate();
	const toast = useToast();
	const [workspaceId] = useLastWorkspaceId();
	const queryClient = useQueryClient();
	const canModifyMembers = useIsAllowed("modifyRoles", "edit");

	const workspaceInfoQuery = useWorkspaceInfoQuery();
	const rolesQuery = useAllRolesQuery(
		() => undefined,
		() => "100"
	);

	const [email, setEmail] = createSignal("");
	const [bindings, setBindings] = createSignal<Binding[]>([{ subjectId: "", scope: { scopeType: "workspace" } }]);

	const roleOptions = createMemo(() =>
		(rolesQuery.data?.roles ?? []).map((role) => ({ label: role.name, value: role.id }))
	);

	const grants = createMemo<RoleBindingGrant[]>(() =>
		bindings()
			.filter((binding) => binding.subjectId)
			.flatMap((binding) =>
				scopeResources(binding.scope, workspaceId() ?? "").map((resourceId) => ({
					roleId: binding.subjectId,
					resourceId,
				}))
			)
	);

	// A resource-scoped grant with nothing selected reaches nothing.
	const hasEmptyResourceScope = createMemo(() =>
		bindings().some(
			(binding) =>
				binding.subjectId && binding.scope.scopeType === "resources" && binding.scope.resources.length === 0
		)
	);

	const backToMembers = () => navigate({ to: "/workspace/members" });

	const { onSubmit, isLoading } = createFormAction(
		async ({ workspaceId }) => {
			const requestBody: InviteUserToWorkspaceRequest = {
				email: email().trim(),
				roles: grants(),
			};

			const response = await httpRequest<InviteUserToWorkspaceResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/rbac/user/invite`,
				{ method: "POST", body: JSON.stringify(requestBody) }
			);

			if (!response.ok) {
				const err = response.data.error;
				toast(
					err === "userAlreadyMember"
						? "That email already belongs to a member of this workspace"
						: err === "inviteAlreadyExists"
							? "That email has already been invited — edit or revoke it on the members page"
							: "Failed to send invite",
					"error"
				);
				return;
			}

			queryClient.invalidateQueries({ queryKey: inviteKeys.all(workspaceId) });
			toast("Invite sent", "success");
			// The accept token is returned exactly once — it is stored hashed —
			// so hand it back through history state rather than dropping it on
			// navigation. Not a search param: it would put a live invite token
			// in the URL, the history and any referrer.
			navigate({
				to: "/workspace/members",
				state: (prev) => ({
					...prev,
					newInvite: { id: response.data.id, acceptUrl: response.data.acceptUrl },
				}),
			});
		},
		() => {
			if (!email().trim()) {
				toast("Enter an email address to invite", "error");
				return false;
			}
			if (grants().length === 0) {
				toast("Give the invitee at least one role", "error");
				return false;
			}
			// The backend rejects this too, but saying so here beats a 400.
			if (hasEmptyResourceScope()) {
				toast("A role scoped to specific resources needs at least one resource", "error");
				return false;
			}
			return true;
		}
	);

	return (
		<>
			<Title>Invite Member | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{ label: "Workspace Settings", url: "/workspace" },
						{ label: "Members", url: "/workspace/members" },
						{ label: "Invite" },
					]}
					subText={`Invite someone to ${workspaceInfoQuery.data?.name ?? "this workspace"} and choose what they can reach.`}
				/>
				<PageContainerBody>
					<Show
						when={canModifyMembers()}
						fallback={
							<div class="p-6 bg-secondary-light text-error rounded-xs text-sm">
								You don't have permission to invite people to this workspace.
							</div>
						}
					>
						<form class="flex flex-col gap-6 max-w-5xl" onSubmit={onSubmit}>
							<div class="flex flex-col gap-2">
								<Label label="Email address" />
								<Input
									type={InputType.Email}
									placeholder="someone@example.com"
									value={email()}
									onInput={(e) => setEmail(e.currentTarget.value)}
								/>
							</div>

							<div class="flex flex-col gap-2">
								<Label label="Access" />
								<p class="text-grey text-xs">
									Each row grants one role, either across the whole workspace or on the resources you
									pick.
								</p>
								<BindingRows
									workspaceId={workspaceId()!}
									bindings={bindings()}
									onChange={setBindings}
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
							</div>

							<div class="flex items-center gap-2">
								<Button
									type="submit"
									variant={ButtonVariant.Contained}
									class="flex items-center gap-2"
									disabled={isLoading()}
									loading={isLoading}
									loadingContent={() => <span>Sending...</span>}
								>
									<FiSend size={14} />
									Send invite
								</Button>
								<Button
									variant={ButtonVariant.Outlined}
									class="flex items-center gap-2"
									onClick={backToMembers}
								>
									<FiArrowLeft size={14} />
									Cancel
								</Button>
							</div>
						</form>
					</Show>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/workspace/members_/invite")({
	component: InviteMember,
});
