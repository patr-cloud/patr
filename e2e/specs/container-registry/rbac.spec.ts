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
import { createContainerRepo } from '@/helpers/registry';
import {
	openRegistryList,
	createRepoLink,
	repoRow,
	emptyStateHeading,
} from '@/helpers/ui/container-registry';

// Registry RBAC at the API layer (view/create/delete gating, Create≠View,
// membership-gated empty list, cross-workspace isolation, scoped API tokens)
// lives in the Rust API suite
// (api/tests/api/workspace/rbac/permissions/container_registry.rs and
// api/tests/api/user/api_token.rs). Here we cover dashboard control-visibility.

type Owner = UserHandle & { workspaceId: string };

async function permId(api: ApiClient, owner: Owner, name: string): Promise<string> {
	return getPermissionId(api, owner.accessToken, owner.workspaceId, owner.clientIp, name);
}

// Control-visibility through the dashboard: the Create Repository CTA is gated
// on the create permission.
test.describe('container registry > RBAC [UI]', () => {
	test('a view-only member sees repos but no Create Repository CTA', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const repo = await createContainerRepo(api, owner, owner.workspaceId);
		const viewId = await permId(api, owner, 'containerRegistryRepository::view');
		await using member = await createSecondMemberWithRole(api, owner, [viewId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openRegistryList(page);
			await expect(repoRow(page, repo.name)).toBeVisible({ timeout: 15_000 });
			await expect(createRepoLink(page)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	test('a member with no registry permission sees the empty state', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await createContainerRepo(api, owner, owner.workspaceId);
		const viewRoles = await permId(api, owner, 'viewRoles');
		await using member = await createSecondMemberWithRole(api, owner, [viewRoles]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openRegistryList(page);
			await expect(emptyStateHeading(page)).toBeVisible({ timeout: 15_000 });
		} finally {
			await context.close();
		}
	});
});
