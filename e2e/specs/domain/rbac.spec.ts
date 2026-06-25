import {
  test,
  expect,
  newContext,
  loginAs,
  createUserWithWorkspace,
  createSecondMemberWithRole,
  getPermissionId,
} from '@/prelude';
import type { ApiClient, UserHandle } from '@/prelude';
import { addDomainAPI } from '@/helpers/domain-api';
import { openDomainList, addDomainLink, domainRow, emptyStateHeading } from '@/helpers/ui/domain';

// Domain RBAC at the API layer (view/add/verify/delete gating, Add≠View,
// membership-gated empty list, cross-workspace isolation) lives in the Rust API
// suite (api/tests/api/workspace/rbac/permissions/domain.rs). Here we cover the
// dashboard control-visibility surface only.

type Owner = UserHandle & { workspaceId: string };

async function permId(api: ApiClient, owner: Owner, name: string): Promise<string> {
  return getPermissionId(api, owner.accessToken, owner.workspaceId, owner.clientIp, name);
}

// Control-visibility through the dashboard: the Add Domain CTA is gated on the
// add permission.
test.describe('domain > RBAC [UI]', () => {
  test('a view-only member sees domains but no Add Domain CTA', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    const added = await addDomainAPI(api, owner, owner.workspaceId);
    const viewId = await permId(api, owner, 'domain::view');
    await using member = await createSecondMemberWithRole(api, owner, {
      [viewId]: { permissionType: 'exclude', resources: [] },
    });
    const context = await newContext(browser, member.clientIp);
    await loginAs(context, member, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await openDomainList(page);
      await expect(domainRow(page, added.domain)).toBeVisible({ timeout: 15_000 });
      await expect(addDomainLink(page)).toHaveCount(0);
    } finally {
      await context.close();
    }
  });

  test('a member with no domain permission sees the empty state', async ({ browser, api }) => {
    await using owner = await createUserWithWorkspace(api);
    await addDomainAPI(api, owner, owner.workspaceId);
    const viewRoles = await permId(api, owner, 'viewRoles');
    await using member = await createSecondMemberWithRole(api, owner, {
      [viewRoles]: { permissionType: 'exclude', resources: [] },
    });
    const context = await newContext(browser, member.clientIp);
    await loginAs(context, member, { workspaceId: owner.workspaceId });
    const page = await context.newPage();
    try {
      await openDomainList(page);
      await expect(emptyStateHeading(page)).toBeVisible({ timeout: 15_000 });
    } finally {
      await context.close();
    }
  });
});
