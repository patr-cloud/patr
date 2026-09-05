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
import { createRunnerAPI } from '@/helpers/runner-api';
import {
	openRunnerList,
	addRunnerLink,
	runnerRow,
	emptyStateHeading,
	openRunnerDetail,
} from '@/helpers/ui/runner';
import { noPermissionsHeading } from '@/helpers/ui/deployment';

// Runner RBAC at the API layer (view/create/delete gating, Create≠View,
// ingress-token requires Execute, membership-gated empty list, cross-workspace
// isolation, scoped API tokens) lives in the Rust API suite
// (api/tests/api/workspace/rbac/permissions/runner.rs and
// api/tests/api/user/api_token.rs). Here we cover dashboard control-visibility.

type Owner = UserHandle & { workspaceId: string };

async function permId(api: ApiClient, owner: Owner, name: string): Promise<string> {
	return getPermissionId(api, owner.accessToken, owner.workspaceId, owner.clientIp, name);
}

// Control-visibility through the dashboard: the create CTA is gated on the
// create permission, so a view-only member sees runners but no "Add Runner".
test.describe('runner > RBAC [UI]', () => {
	test('a view-only member sees runners but no Add Runner CTA', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, owner, owner.workspaceId);
		const viewId = await permId(api, owner, 'runner::view');
		await using member = await createSecondMemberWithRole(api, owner, [viewId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openRunnerList(page);
			await expect(runnerRow(page, runner.name)).toBeVisible({ timeout: 15_000 });
			await expect(addRunnerLink(page)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	test('a member with the create permission sees the Add Runner CTA', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const createId = await permId(api, owner, 'runner::create');
		await using member = await createSecondMemberWithRole(api, owner, [createId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openRunnerList(page);
			await expect(addRunnerLink(page)).toBeVisible({ timeout: 15_000 });
		} finally {
			await context.close();
		}
	});

	test('a view member reaches a runner detail instead of NoPermissionsPage', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, owner, owner.workspaceId);
		const viewId = await permId(api, owner, 'runner::view');
		await using member = await createSecondMemberWithRole(api, owner, [viewId]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openRunnerDetail(page, runner.id);
			await expect(noPermissionsHeading(page)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	test('a member with no runner permission sees the empty state', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await createRunnerAPI(api, owner, owner.workspaceId);
		const viewRoles = await permId(api, owner, 'viewRoles');
		await using member = await createSecondMemberWithRole(api, owner, [viewRoles]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openRunnerList(page);
			await expect(emptyStateHeading(page)).toBeVisible({ timeout: 15_000 });
		} finally {
			await context.close();
		}
	});
});
