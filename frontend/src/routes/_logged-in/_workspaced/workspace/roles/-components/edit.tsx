import { Alert, Button, ButtonVariant, Input, UnsavedChangesGuard, useToast } from "~/components";
import { Color } from "~/utils/color";
import PermissionPicker from "./permission-picker";
import { createEffect, createMemo, createSignal, Show, Suspense } from "solid-js";
import { useParams } from "@tanstack/solid-router";
import { httpRequest } from "~/utils/http-request";
import { UpdateRoleRequest } from "~/bindings/UpdateRoleRequest";
import { createLoggedInAction } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { useRoleInfoQuery } from "~/hooks/fetch";
import { roleKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import { validateNameField, validateRoleDescription } from "~/utils/validation";

const EditPermissions = () => {
	const [permissions, setPermissions] = createSignal<Set<string>>(new Set());
	const [roleName, setRoleName] = createSignal("");
	const [roleDescription, setRoleDescription] = createSignal("");
	const [roleNameError, setRoleNameError] = createSignal<string | undefined>(undefined);
	const [roleDescriptionError, setRoleDescriptionError] = createSignal<string | undefined>(undefined);
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const queryClient = useQueryClient();
	const params = useParams({ from: "/_logged-in/_workspaced/workspace/roles/$roleId" });

	const roleInfoQuery = useRoleInfoQuery(() => params().roleId);
	const isImmutable = () => roleInfoQuery.data?.isImmutable ?? false;

	// Initialize form state when role info loads
	createEffect(() => {
		const role = roleInfoQuery.data;
		if (role) {
			setPermissions(new Set(role.permissions));
			setRoleName(role.name);
			setRoleDescription(role.description ?? "");
		}
	});

	const { execute: handleUpdateRole, isLoading: isUpdating } = createLoggedInAction(async () => {
		const nameError = validateNameField(roleName());
		const descError = validateRoleDescription(roleDescription());
		setRoleNameError(nameError);
		setRoleDescriptionError(descError);
		if (nameError || descError) return;

		if (permissions().size === 0) {
			toast("Add at least one permission before saving", "error");
			return;
		}

		const requestBody: Omit<UpdateRoleRequest, "isImmutable"> = {
			name: roleName().trim(),
			description: roleDescription().trim(),
			permissions: [...permissions()],
		};

		const response = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/rbac/role/${params().roleId}`,
			{
				method: "PATCH",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			console.error("Failed to update role:", response.data.error);
			toast(response.data.error || "Failed to update role", "error");
			return;
		}

		toast("Role updated successfully", "success");
		const wsId = workspaceId();
		if (wsId) {
			queryClient.invalidateQueries({ queryKey: roleKeys.detail(wsId, params().roleId) });
		}
	});

	// Order-independent serialization of a permission set, so array order from
	// the server never reads as a change against the loaded snapshot.
	const canonicalPermissions = (ids: Iterable<string>) => JSON.stringify([...ids].sort());

	// True while the form differs from what the server last returned. Once a save
	// lands and the query refetches, the seed effect above re-syncs the signals,
	// so this drops back to false on its own.
	const isDirty = createMemo(() => {
		const role = roleInfoQuery.data;
		if (!role || isImmutable()) return false;
		if (roleName().trim() !== role.name) return true;
		if (roleDescription().trim() !== (role.description ?? "")) return true;
		return canonicalPermissions(permissions()) !== canonicalPermissions(role.permissions ?? []);
	});

	return (
		<Suspense fallback={<div class="text-gray-400 text-center py-8">Loading role information...</div>}>
			<div class="flex flex-col gap-4">
				<div class="flex justify-between items-center">
					<h3 class="text-lg text-white">{isImmutable() ? "View Role" : "Edit Role"}</h3>

					<Show when={!isImmutable()}>
						<div class="flex justify-end gap-4">
							<Button
								variant={ButtonVariant.Contained}
								onClick={() => handleUpdateRole().catch(() => {})}
								disabled={isUpdating() || permissions().size === 0}
							>
								{isUpdating() ? "Saving Changes..." : "Save Changes"}
							</Button>
						</div>
					</Show>
				</div>

				<Show when={isImmutable()}>
					<Alert message="This is a built-in role and cannot be edited or deleted." type="warning" />
				</Show>

				<div class="flex flex-col gap-2">
					<label class="text-white text-sm">Role Name</label>
					<Input
						type="text"
						placeholder="Enter Name"
						value={roleName()}
						disabled={isImmutable()}
						onInput={(e) => {
							setRoleName(e.currentTarget.value);
							setRoleNameError(undefined);
						}}
					/>
					<Show when={roleNameError()}>
						<Alert message={roleNameError()!} type="error" />
					</Show>
				</div>

				<div class="flex flex-col gap-2">
					<label class="text-white text-sm">Description</label>
					<Input
						type="text"
						placeholder="Enter Description (optional)"
						value={roleDescription()}
						disabled={isImmutable()}
						onInput={(e) => {
							setRoleDescription(e.currentTarget.value);
							setRoleDescriptionError(undefined);
						}}
					/>
					<Show when={roleDescriptionError()}>
						<Alert message={roleDescriptionError()!} type="error" />
					</Show>
				</div>

				<div class="flex flex-col gap-2">
					<div class="flex justify-between items-center">
						<label class="text-white text-sm">
							{isImmutable() ? "Permissions in role" : "Edit Permissions in role"}
						</label>
						<Show when={!isImmutable()}>
							<Button
								variant={ButtonVariant.Plain}
								color={Color.Error}
								onClick={() => setPermissions(new Set())}
							>
								Clear All
							</Button>
						</Show>
					</div>
					<PermissionPicker
						workspaceId={workspaceId()!}
						selected={permissions()}
						onChange={(next) => setPermissions(next)}
						disabled={isImmutable()}
						// Ticks on load and on the refetch after a save, which is when the
						// picker re-ranks its columns. Local edits leave it untouched, so
						// the cards don't move while you work.
						sortToken={roleInfoQuery.dataUpdatedAt}
					/>
				</div>

				<UnsavedChangesGuard
					when={isDirty}
					message="You have unsaved changes to this role. If you leave now, they'll be lost."
				/>
			</div>
		</Suspense>
	);
};

export default EditPermissions;
