import { Alert, Button, ButtonVariant, Input, UnsavedChangesGuard, useToast } from "~/components";
import { Color } from "~/utils/color";
import PermissionMatrix from "./permission-matrix";
import { createEffect, createMemo, createSignal, Show, Suspense } from "solid-js";
import { useParams } from "@tanstack/solid-router";
import { httpRequest } from "~/utils/http-request";
import { UpdateRoleRequest } from "~/bindings/UpdateRoleRequest";
import { createLoggedInAction } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { ResourcePermissionType } from "~/bindings";
import { useRoleInfoQuery } from "~/hooks/fetch";
import { roleKeys } from "~/hooks/query-keys";
import { useQueryClient } from "@tanstack/solid-query";
import { validateNameField, validateRoleDescription } from "~/utils/validation";

const EditPermissions = () => {
	const [permissionsData, setPermissionsData] = createSignal<{ [key: string]: ResourcePermissionType }>({});
	const [roleName, setRoleName] = createSignal("");
	const [roleDescription, setRoleDescription] = createSignal("");
	const [roleNameError, setRoleNameError] = createSignal<string | undefined>(undefined);
	const [roleDescriptionError, setRoleDescriptionError] = createSignal<string | undefined>(undefined);
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const queryClient = useQueryClient();
	const params = useParams({ from: "/_logged-in/_workspaced/workspace/roles/$roleId" });

	const roleInfoQuery = useRoleInfoQuery(() => params().roleId);

	// Initialize permissions data when role info loads
	createEffect(() => {
		const role = roleInfoQuery.data;
		if (role) {
			setPermissionsData(role.permissions as { [key: string]: ResourcePermissionType });
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

		if (Object.keys(permissionsData()).length === 0) {
			toast("Add at least one permission before saving", "error");
			return;
		}

		const requestBody: UpdateRoleRequest = {
			name: roleName().trim(),
			description: roleDescription().trim(),
			permissions: permissionsData(),
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

	// Order-independent serialization of a permission map, so that toggling a
	// resource off and on again (which can reorder the arrays) doesn't read as a
	// change against the loaded snapshot.
	const canonicalPermissions = (perms: { [key: string]: ResourcePermissionType }) =>
		JSON.stringify(
			Object.keys(perms)
				.sort()
				.map((id) => [id, perms[id].permissionType, [...perms[id].resources].sort()])
		);

	// True while the form differs from what the server last returned. Once a save
	// lands and the query refetches, the seed effect above re-syncs the signals,
	// so this drops back to false on its own.
	const isDirty = createMemo(() => {
		const role = roleInfoQuery.data;
		if (!role) return false;
		if (roleName().trim() !== role.name) return true;
		if (roleDescription().trim() !== (role.description ?? "")) return true;
		return (
			canonicalPermissions(permissionsData()) !==
			canonicalPermissions((role.permissions ?? {}) as { [key: string]: ResourcePermissionType })
		);
	});

	return (
		<Suspense fallback={<div class="text-gray-400 text-center py-8">Loading role information...</div>}>
			<div class="flex flex-col gap-4">
				<div class="flex justify-between items-center">
					<h3 class="text-lg text-white">Edit Role</h3>

					<div class="flex justify-end gap-4">
						<Button
							variant={ButtonVariant.Contained}
							onClick={() => handleUpdateRole().catch(() => {})}
							disabled={isUpdating() || Object.keys(permissionsData()).length === 0}
						>
							{isUpdating() ? "Saving Changes..." : "Save Changes"}
						</Button>
					</div>
				</div>

				<div class="flex flex-col gap-2">
					<label class="text-white text-sm">Role Name</label>
					<Input
						type="text"
						placeholder="Enter Name"
						value={roleName()}
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
						<label class="text-white text-sm">Edit Permissions in role</label>
						<Button
							variant={ButtonVariant.Plain}
							color={Color.Error}
							onClick={() => setPermissionsData({})}
						>
							Clear All
						</Button>
					</div>
					<PermissionMatrix
						workspaceId={workspaceId()!}
						permissionsData={permissionsData()}
						onChange={(next) => setPermissionsData(next)}
						// Ticks on load and on the refetch after a save, which is when the
						// matrix re-ranks its columns. Local edits leave it untouched, so
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
