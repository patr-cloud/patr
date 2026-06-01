import type { ApiClient } from '@/helpers/api';

export type PermissionInput = {
  permissionType: 'include' | 'exclude';
  resources: string[];
};

// Resolve permission name (e.g. "deployment::view" or "modifyRoles") to its
// UUID. Cached per (workspaceId, name) pair within a single test run — names
// are stable but ids regenerate per DB init.
const permissionIdCache = new Map<string, string>();

export async function getPermissionId(
  api: ApiClient,
  token: string,
  wsId: string,
  clientIp: string,
  name: string,
): Promise<string> {
  const key = `${wsId}::${name}`;
  const cached = permissionIdCache.get(key);
  if (cached) return cached;
  const resp = await api.request<{
    permissions: { id: string; name: string }[];
  }>('GET', `/workspace/${wsId}/rbac/permission`, { token, clientIp });
  for (const p of resp.permissions) {
    permissionIdCache.set(`${wsId}::${p.name}`, p.id);
  }
  const id = permissionIdCache.get(key);
  if (!id) throw new Error(`Unknown permission: ${name}`);
  return id;
}

export async function listPermissions(
  api: ApiClient,
  token: string,
  wsId: string,
  clientIp: string,
): Promise<{ id: string; name: string }[]> {
  const resp = await api.request<{
    permissions: { id: string; name: string }[];
  }>('GET', `/workspace/${wsId}/rbac/permission`, { token, clientIp });
  return resp.permissions;
}

export async function createRoleAPI(
  api: ApiClient,
  user: { accessToken: string; clientIp: string },
  wsId: string,
  body: {
    name: string;
    description?: string;
    permissions: Record<string, PermissionInput>;
  },
): Promise<{ id: string }> {
  return api.request<{ id: string }>('POST', `/workspace/${wsId}/rbac/role`, {
    token: user.accessToken,
    clientIp: user.clientIp,
    body: {
      name: body.name,
      description: body.description ?? `Role: ${body.name}`,
      permissions: body.permissions,
    },
  });
}

export async function updateRoleAPI(
  api: ApiClient,
  user: { accessToken: string; clientIp: string },
  wsId: string,
  roleId: string,
  body: Partial<{
    name: string;
    description: string;
    permissions: Record<string, PermissionInput>;
  }>,
): Promise<void> {
  await api.request('PATCH', `/workspace/${wsId}/rbac/role/${roleId}`, {
    token: user.accessToken,
    clientIp: user.clientIp,
    body,
  });
}

export async function deleteRoleAPI(
  api: ApiClient,
  user: { accessToken: string; clientIp: string },
  wsId: string,
  roleId: string,
  opts: { removeUsers?: boolean } = {},
): Promise<void> {
  // Serde renames the query field to camelCase, so the URL uses `removeUsers`
  // (not snake_case `remove_users`).
  const qs = opts.removeUsers ? '?removeUsers=true' : '';
  await api.request('DELETE', `/workspace/${wsId}/rbac/role/${roleId}${qs}`, {
    token: user.accessToken,
    clientIp: user.clientIp,
  });
}

export async function listRolesAPI(
  api: ApiClient,
  user: { accessToken: string; clientIp: string },
  wsId: string,
): Promise<{ id: string; name: string; description: string }[]> {
  const resp = await api.request<{
    roles: { id: string; name: string; description: string }[];
  }>('GET', `/workspace/${wsId}/rbac/role?page=0&count=100`, {
    token: user.accessToken,
    clientIp: user.clientIp,
  });
  return resp.roles;
}

export async function getRoleAPI(
  api: ApiClient,
  user: { accessToken: string; clientIp: string },
  wsId: string,
  roleId: string,
): Promise<{
  id: string;
  name: string;
  description: string;
  permissions: Record<string, PermissionInput>;
}> {
  return api.request('GET', `/workspace/${wsId}/rbac/role/${roleId}`, {
    token: user.accessToken,
    clientIp: user.clientIp,
  });
}

export async function setUserRolesAPI(
  api: ApiClient,
  user: { accessToken: string; clientIp: string },
  wsId: string,
  userId: string,
  roleIds: string[],
): Promise<void> {
  await api.request('POST', `/workspace/${wsId}/rbac/user/${userId}`, {
    token: user.accessToken,
    clientIp: user.clientIp,
    body: { roles: roleIds },
  });
}

export async function removeMemberAPI(
  api: ApiClient,
  user: { accessToken: string; clientIp: string },
  wsId: string,
  userId: string,
): Promise<void> {
  await api.request('DELETE', `/workspace/${wsId}/rbac/user/${userId}`, {
    token: user.accessToken,
    clientIp: user.clientIp,
  });
}

export async function currentPermissionsAPI(
  api: ApiClient,
  user: { accessToken: string; clientIp: string },
  wsId: string,
): Promise<unknown> {
  return api.request('GET', `/workspace/${wsId}/rbac/current-permissions`, {
    token: user.accessToken,
    clientIp: user.clientIp,
  });
}
