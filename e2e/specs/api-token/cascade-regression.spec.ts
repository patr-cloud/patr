// Locks in the behaviour shipped by the cascade fix:
//   - `update_user_roles_in_workspace` cascade: revoking a member's roles
//     trims any API tokens that previously inherited those perms (already
//     covered for the auth path by permissions.spec.ts; this spec adds the
//     `get_api_token_info` round-trip + DB-rewrite assertion).
//   - `delete_role` with `remove_users=true` cascade: deleting a role that
//     was the sole source of a perm trims the token on its next use.
//   - Monotonic-shrink invariant: promoting a member's role does NOT widen
//     the perms of an existing token. Tokens only ever shrink.
import {
  test,
  expect,
  createUserWithWorkspace,
  createUserAccount,
  createApiTokenAPI,
  callWithApiToken,
  addMemberToWorkspace,
} from '@/prelude';
import {
  createRoleAPI,
  deleteRoleAPI,
  setUserRolesAPI,
  getPermissionId,
} from '@/helpers/api/rbac';

test.describe('api token cascade regression', () => {
  test('update_user_roles cascade: GET token info reflects the trim', async ({
    api,
  }) => {
    await using owner = await createUserWithWorkspace(api);
    await using memberB = await createUserAccount(api);

    const modifyRolesId = await getPermissionId(
      api,
      owner.accessToken,
      owner.workspaceId,
      owner.clientIp,
      'modifyRoles',
    );
    const viewRolesId = await getPermissionId(
      api,
      owner.accessToken,
      owner.workspaceId,
      owner.clientIp,
      'viewRoles',
    );

    // Give B a role with modifyRoles, mint a token with that perm.
    const role = await createRoleAPI(api, owner, owner.workspaceId, {
      name: `cascade-role-${Date.now().toString(36)}`,
      permissions: {
        [modifyRolesId]: { permissionType: 'exclude', resources: [] },
      },
    });
    await addMemberToWorkspace(api, owner, owner.workspaceId, memberB, [role.id]);

    const token = await createApiTokenAPI(api, memberB, {
      permissions: {
        [owner.workspaceId]: {
          type: 'member',
          [modifyRolesId]: { permissionType: 'exclude', resources: [] },
        } as any,
      },
    });

    // Strip B's role entirely. modifyRoles is gone from the user.
    const memberBId = (
      await api.request<{ id: string }>('GET', '/user', {
        token: memberB.accessToken,
        clientIp: memberB.clientIp,
      })
    ).id;
    await setUserRolesAPI(api, owner, owner.workspaceId, memberBId, []);

    // First use of the token after the trim auths against modifyRoles → 401
    // (covered by permissions.spec.ts). This spec ALSO confirms the
    // get_api_token_info response now reflects the trim — meaning the cascade
    // wrote back to the DB on the cache-miss path.
    const callRes = await callWithApiToken(api, token.token, {
      clientIp: memberB.clientIp,
      method: 'POST',
      path: `/workspace/${owner.workspaceId}/rbac/role`,
      body: {
        name: `tmp-${Date.now().toString(36)}`,
        description: 'cascade-probe',
        permissions: {
          [viewRolesId]: { permissionType: 'exclude', resources: [] },
        },
      },
    });
    expect(callRes.status).toBe(401);

    type TokenInfo = {
      permissions: Record<string, { type: string } & Record<string, unknown>>;
    };
    const info = await api.request<TokenInfo>(
      'GET',
      `/user/api-token/${token.id}`,
      { token: memberB.accessToken, clientIp: memberB.clientIp },
    );
    const wsPerm = info.permissions[owner.workspaceId];
    if (wsPerm && wsPerm.type === 'member') {
      // After the cascade-trim the workspace entry should either be dropped
      // or remain as Member with no modifyRoles permission.
      expect(wsPerm[modifyRolesId]).toBeUndefined();
    }
  });

  test('delete_role with removeUsers=true cascades to existing tokens', async ({
    api,
  }) => {
    await using owner = await createUserWithWorkspace(api);
    await using memberB = await createUserAccount(api);

    const modifyRolesId = await getPermissionId(
      api,
      owner.accessToken,
      owner.workspaceId,
      owner.clientIp,
      'modifyRoles',
    );
    const viewRolesId = await getPermissionId(
      api,
      owner.accessToken,
      owner.workspaceId,
      owner.clientIp,
      'viewRoles',
    );

    const role = await createRoleAPI(api, owner, owner.workspaceId, {
      name: `delete-cascade-role-${Date.now().toString(36)}`,
      permissions: {
        [modifyRolesId]: { permissionType: 'exclude', resources: [] },
      },
    });
    await addMemberToWorkspace(api, owner, owner.workspaceId, memberB, [role.id]);

    const token = await createApiTokenAPI(api, memberB, {
      permissions: {
        [owner.workspaceId]: {
          type: 'member',
          [modifyRolesId]: { permissionType: 'exclude', resources: [] },
        } as any,
      },
    });

    // Confirm the token works first.
    const probe = await callWithApiToken(api, token.token, {
      clientIp: memberB.clientIp,
      method: 'POST',
      path: `/workspace/${owner.workspaceId}/rbac/role`,
      body: {
        name: `pre-${Date.now().toString(36)}`,
        description: 'pre-probe',
        permissions: {
          [viewRolesId]: { permissionType: 'exclude', resources: [] },
        },
      },
    });
    expect(probe.status).toBe(201);

    // Delete the role with removeUsers=true → user_id_revocation_timestamp
    // (via the workspace bump) invalidates the token's perm cache. Next call
    // through the auth path re-reads from the DB, intersects with the user's
    // now-empty role-derived perms, and trims.
    await deleteRoleAPI(api, owner, owner.workspaceId, role.id, {
      removeUsers: true,
    });

    const after = await callWithApiToken(api, token.token, {
      clientIp: memberB.clientIp,
      method: 'POST',
      path: `/workspace/${owner.workspaceId}/rbac/role`,
      body: {
        name: `post-${Date.now().toString(36)}`,
        description: 'post-probe',
        permissions: {
          [viewRolesId]: { permissionType: 'exclude', resources: [] },
        },
      },
    });
    expect(after.status).toBe(401);
  });

  test('monotonic shrink: promoting a member does NOT widen an existing token', async ({
    api,
  }) => {
    await using owner = await createUserWithWorkspace(api);
    await using memberB = await createUserAccount(api);

    const viewRolesId = await getPermissionId(
      api,
      owner.accessToken,
      owner.workspaceId,
      owner.clientIp,
      'viewRoles',
    );
    const modifyRolesId = await getPermissionId(
      api,
      owner.accessToken,
      owner.workspaceId,
      owner.clientIp,
      'modifyRoles',
    );

    // B has a read-only role (viewRoles only).
    const readOnly = await createRoleAPI(api, owner, owner.workspaceId, {
      name: `view-only-${Date.now().toString(36)}`,
      permissions: {
        [viewRolesId]: { permissionType: 'exclude', resources: [] },
      },
    });
    await addMemberToWorkspace(api, owner, owner.workspaceId, memberB, [readOnly.id]);

    // B mints a token with viewRoles only.
    const token = await createApiTokenAPI(api, memberB, {
      permissions: {
        [owner.workspaceId]: {
          type: 'member',
          [viewRolesId]: { permissionType: 'exclude', resources: [] },
        } as any,
      },
    });

    // Owner promotes B by swapping in a role with modifyRoles too.
    const writeRole = await createRoleAPI(api, owner, owner.workspaceId, {
      name: `view-and-modify-${Date.now().toString(36)}`,
      permissions: {
        [viewRolesId]: { permissionType: 'exclude', resources: [] },
        [modifyRolesId]: { permissionType: 'exclude', resources: [] },
      },
    });
    const memberBId = (
      await api.request<{ id: string }>('GET', '/user', {
        token: memberB.accessToken,
        clientIp: memberB.clientIp,
      })
    ).id;
    await setUserRolesAPI(api, owner, owner.workspaceId, memberBId, [writeRole.id]);

    // Token's declared perms only ever had viewRoles. After promotion the
    // intersection is still viewRoles (declared ∩ current = declared).
    // A modifyRoles call via the token must remain 401.
    const callRes = await callWithApiToken(api, token.token, {
      clientIp: memberB.clientIp,
      method: 'POST',
      path: `/workspace/${owner.workspaceId}/rbac/role`,
      body: {
        name: `widen-probe-${Date.now().toString(36)}`,
        description: 'widen-probe',
        permissions: {
          [viewRolesId]: { permissionType: 'exclude', resources: [] },
        },
      },
    });
    expect(callRes.status).toBe(401);
  });
});
