import {
  test,
  expect,
  createUserAccount,
  createUserWithWorkspace,
  createRoleAPI,
  getPermissionId,
  getOwnUserId,
} from '@/prelude';

test.describe('rbac > cross-workspace isolation', () => {
  test('rejects GET /rbac/role/{id} via a different workspace URL', async ({ api }) => {
    await using ownerA = await createUserWithWorkspace(api);
    await using ownerB = await createUserWithWorkspace(api);
    const viewIdA = await getPermissionId(
      api,
      ownerA.accessToken,
      ownerA.workspaceId,
      ownerA.clientIp,
      'viewRoles',
    );
    const roleA = await createRoleAPI(api, ownerA, ownerA.workspaceId, {
      name: `isoA-${Date.now().toString(36)}`,
      permissions: { [viewIdA]: { permissionType: 'exclude', resources: [] } },
    });
    // ownerB tries to GET A's role via B's workspace URL.
    await expect(
      api.request('GET', `/workspace/${ownerB.workspaceId}/rbac/role/${roleA.id}`, {
        token: ownerB.accessToken,
        clientIp: ownerB.clientIp,
      }),
    ).rejects.toThrow(/4\d\d/);
  });

  test('rejects PATCH /rbac/role/{id} via a different workspace URL', async ({ api }) => {
    await using ownerA = await createUserWithWorkspace(api);
    await using ownerB = await createUserWithWorkspace(api);
    const viewIdA = await getPermissionId(
      api,
      ownerA.accessToken,
      ownerA.workspaceId,
      ownerA.clientIp,
      'viewRoles',
    );
    const roleA = await createRoleAPI(api, ownerA, ownerA.workspaceId, {
      name: `isoP-${Date.now().toString(36)}`,
      permissions: { [viewIdA]: { permissionType: 'exclude', resources: [] } },
    });
    await expect(
      api.request('PATCH', `/workspace/${ownerB.workspaceId}/rbac/role/${roleA.id}`, {
        token: ownerB.accessToken,
        clientIp: ownerB.clientIp,
        body: { description: 'x' },
      }),
    ).rejects.toThrow(/4\d\d/);
  });

  test('rejects DELETE /rbac/role/{id} via a different workspace URL', async ({ api }) => {
    await using ownerA = await createUserWithWorkspace(api);
    await using ownerB = await createUserWithWorkspace(api);
    const viewIdA = await getPermissionId(
      api,
      ownerA.accessToken,
      ownerA.workspaceId,
      ownerA.clientIp,
      'viewRoles',
    );
    const roleA = await createRoleAPI(api, ownerA, ownerA.workspaceId, {
      name: `isoD-${Date.now().toString(36)}`,
      permissions: { [viewIdA]: { permissionType: 'exclude', resources: [] } },
    });
    await expect(
      api.request('DELETE', `/workspace/${ownerB.workspaceId}/rbac/role/${roleA.id}`, {
        token: ownerB.accessToken,
        clientIp: ownerB.clientIp,
      }),
    ).rejects.toThrow(/4\d\d/);
  });

  test('refuses to add a member to a workspace the caller does not own', async ({ api }) => {
    await using ownerA = await createUserWithWorkspace(api);
    await using ownerB = await createUserWithWorkspace(api);
    await using outsider = await createUserAccount(api);
    const outsiderId = await getOwnUserId(api, outsider);
    // ownerB tries to add outsider to workspace A.
    await expect(
      api.request('POST', `/workspace/${ownerA.workspaceId}/rbac/user/${outsiderId}`, {
        token: ownerB.accessToken,
        clientIp: ownerB.clientIp,
        body: { roles: [] },
      }),
    ).rejects.toThrow(/4\d\d/);
  });
});
