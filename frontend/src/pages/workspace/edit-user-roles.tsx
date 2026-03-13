import { createSignal, For, Show } from "solid-js";
import { Button, ButtonVariant, InputDropdown, useToast } from "~/components";
import { FiX, FiTrash2 } from "solid-icons/fi";
import { httpRequest } from "~/utils/http-request";
import { UpdateUserRolesInWorkspaceRequest } from "~/bindings/UpdateUserRolesInWorkspaceRequest";
import { createLoggedInAction } from "~/hooks";

interface EditRolesProps {
	userName: string;
	userId: string;
	workspaceId: string;
	currentRoles: Array<{ id: string; name: string }>;
	availableRoles: Array<{ id: string; name: string }>;
	onSave: (roleIds: string[]) => void;
	onClose: () => void;
}

export const EditUserRoles = (props: EditRolesProps) => {
	const [selectedRoles, setSelectedRoles] = createSignal<string[]>(props.currentRoles.map((r) => r.id));
	const [newRoleId, setNewRoleId] = createSignal("");
	const toast = useToast();

	const handleAddRole = () => {
		const roleId = newRoleId().trim();
		if (roleId && !selectedRoles().includes(roleId)) {
			setSelectedRoles([...selectedRoles(), roleId]);
			setNewRoleId("");
		}
	};

	const handleRemoveRole = (roleId: string) => {
		setSelectedRoles(selectedRoles().filter((id) => id !== roleId));
	};

	const { execute: handleSave, isLoading: isSaving } = createLoggedInAction(async ({ accessToken }) => {
		const requestBody: UpdateUserRolesInWorkspaceRequest = {
			roles: selectedRoles(),
		};

		const response = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${props.workspaceId}/rbac/user/${props.userId}`,
			{
				method: "POST",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			console.error("Failed to update roles:", response.data.error);
			toast("Failed to update roles", "error");
			return;
		}

		toast("Roles updated successfully", "success");
		props.onSave(selectedRoles());
	});

	return (
		<div class="w-full p-md pb-sm bg-secondary-light rounded-xs">
			<div class="flex items-center justify-between pb-4">
				<div class="flex gap-2">
					<h2 class="text-lg text-white">Editing Roles</h2>
					<p class="text-white text-lg">of {props.userName}</p>
				</div>
				<button onClick={props.onClose} class="text-primary text-sm hover:underline cursor-pointer">
					<FiX size={18} />
				</button>
			</div>

			<div class="mb-4">
				<h3 class="text-white text-sm font-medium mb-2">Current Roles</h3>
				<Show
					when={selectedRoles().length > 0}
					fallback={<p class="text-gray-400 text-sm">No roles assigned</p>}
				>
					<div class="grid grid-cols-4 gap-2">
						<For each={selectedRoles()}>
							{(roleId) => {
								const role = props.availableRoles.find((r) => r.id === roleId);
								return (
									<div class="flex items-center justify-between bg-secondary p-2 rounded">
										<span class="text-white text-sm truncate">{role?.name || roleId}</span>
										<button
											onClick={() => handleRemoveRole(roleId)}
											class="text-red-500 hover:text-red-400 transition-colors ml-1 shrink-0"
										>
											<FiTrash2 size={16} />
										</button>
									</div>
								);
							}}
						</For>
					</div>
				</Show>
			</div>

			<div class="mb-4">
				<h3 class="text-white text-sm font-medium mb-2">Add Role</h3>
				<div class="flex gap-2 w-full justify-between">
					<div class="w-[30%] flex gap-2">
						<InputDropdown
							placeholder="Select Role"
							styleVariant="medium"
							options={props.availableRoles
								.filter((role) => !selectedRoles().includes(role.id))
								.map((role) => ({
									label: role.name,
									value: role.id,
								}))}
							value={newRoleId()}
							onSelect={(value) => setNewRoleId(value)}
						/>
						<Button variant={ButtonVariant.Contained} onClick={handleAddRole} disabled={!newRoleId()}>
							Add
						</Button>
					</div>
					<div class="flex gap-3 justify-end">
						<Button variant={ButtonVariant.Outlined} onClick={props.onClose}>
							Cancel
						</Button>
						<Button
							variant={ButtonVariant.Contained}
							onClick={() => handleSave().catch(() => {})}
							loading={isSaving()}
							loadingContent={() => <span>Saving...</span>}
						>
							Save
						</Button>
					</div>
				</div>
			</div>
		</div>
	);
};
