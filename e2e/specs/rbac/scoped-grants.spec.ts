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
import { openRunnerList, runnerRow } from '@/helpers/ui/runner';

// Resource-scoped grants: a role granted on an explicit resource set reaches
// exactly those resources. Exhaustive API-level coverage (every resource type,
// list filtering, union of grants) lives in the Rust API suite
// (api/tests/api/workspace/rbac/permissions/**); here we cover one end-to-end
// slice through the API and the dashboard list.

type Owner = UserHandle & { workspaceId: string };

async function permId(api: ApiClient, owner: Owner, name: string): Promise<string> {
	return getPermissionId(api, owner.accessToken, owner.workspaceId, owner.clientIp, name);
}

test.describe('rbac > resource-scoped grants', () => {
	test('a member scoped to one runner can fetch it but not its sibling', async ({ api }) => {
		await using owner = await createUserWithWorkspace(api);
		const allowed = await createRunnerAPI(api, owner, owner.workspaceId);
		const denied = await createRunnerAPI(api, owner, owner.workspaceId);
		const viewId = await permId(api, owner, 'runner::view');
		await using member = await createSecondMemberWithRole(api, owner, [viewId], [allowed.id]);

		const fetchRunner = (id: string) =>
			api.request('GET', `/workspace/${owner.workspaceId}/runner/${id}`, {
				token: member.accessToken,
				clientIp: member.clientIp,
			});
		await expect(fetchRunner(allowed.id)).resolves.toBeTruthy();
		await expect(fetchRunner(denied.id)).rejects.toThrow(/401/);
	});

	test('the dashboard runner list shows only the runners in scope', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const allowed = await createRunnerAPI(api, owner, owner.workspaceId);
		const denied = await createRunnerAPI(api, owner, owner.workspaceId);
		const viewId = await permId(api, owner, 'runner::view');
		await using member = await createSecondMemberWithRole(api, owner, [viewId], [allowed.id]);
		const context = await newContext(browser, member.clientIp);
		await loginAs(context, member, { workspaceId: owner.workspaceId });
		const page = await context.newPage();
		try {
			await openRunnerList(page);
			await expect(runnerRow(page, allowed.name)).toBeVisible({ timeout: 15_000 });
			await expect(runnerRow(page, denied.name)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});
});
