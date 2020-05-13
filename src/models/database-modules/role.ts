import { roles } from '../interfaces/role';
import { Permission } from '../interfaces/permission';

export function getRoleById(roleId: number) {
	return roles.find((r) => r.roleId === roleId);
}

export function checkIfRoleGrantsPermission(roleId: number, permission: Permission) {
	if (roleId === 0) {
		// roleId 0 implies owner of a resource. An owner can do all permissions
		return true;
	}
	const role = getRoleById(roleId);
	if (role) {
		return role.permissions.indexOf(permission) > -1;
	}
	return false;
}
