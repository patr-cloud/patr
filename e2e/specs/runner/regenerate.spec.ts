import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	callWithApiToken,
	loginAs,
} from '@/prelude';
import { createRunnerAPI } from '@/helpers/runner-api';
import {
	openRunnerDetail,
	regenerateToken,
	readNewRunnerToken,
	newRunnerTokenHeading,
	newRunnerTokenCommand,
	closeNewRunnerTokenDialog,
} from '@/helpers/ui/runner';

// Rotation at the API layer (warm-cache rejection, the old stream closing,
// cross-workspace 404, RBAC) lives in the Rust API suite
// (api/tests/api/workspace/runner.rs and rbac/permissions/runner.rs). Here we
// cover the dashboard flow end to end.

test.describe('runner > regenerate token [UI]', () => {
	test('invalidates the old token and accepts the new one', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		const runnerPath = `/workspace/${user.workspaceId}/runner/${runner.id}`;

		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		let newToken = '';
		try {
			await openRunnerDetail(page, runner.id);
			await regenerateToken(page, runner.name);
			newToken = await readNewRunnerToken(page);
			await expect(
				newRunnerTokenCommand(page, user.workspaceId, runner.id, newToken),
			).toBeVisible();
			expect(page.url()).not.toContain(newToken);
		} finally {
			await context.close();
		}

		expect(newToken).not.toBe(runner.token);
		const oldR = await callWithApiToken(api, runner.token, {
			clientIp: user.clientIp,
			path: runnerPath,
		});
		expect(oldR.status).toBe(401);
		const newR = await callWithApiToken(api, newToken, {
			clientIp: user.clientIp,
			path: runnerPath,
		});
		expect(newR.status).toBe(200);
	});

	test('closing the token dialog leaves the token nowhere on the page', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);

		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openRunnerDetail(page, runner.id);
			await regenerateToken(page, runner.name);
			const newToken = await readNewRunnerToken(page);
			await closeNewRunnerTokenDialog(page);
			await expect(newRunnerTokenHeading(page)).toHaveCount(0);
			await expect(page.getByText(newToken)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});
});
