import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	loginAs,
	expectUrl,
	callWithApiToken,
	scopedTo,
} from '@/prelude';
import { createRunnerAPI } from '@/helpers/runner-api';
import {
	builtInRoleId,
	createServiceAccountAPI,
	getServiceAccountAPI,
	randomServiceAccountName,
} from '@/helpers/service-account-api';
import {
	openServiceAccountDetail,
	nameInput,
	descriptionInput,
	copyableValue,
	addBindingRole,
	removeBindingRole,
	saveButton,
	regenerateToken,
	readNewToken,
	newTokenHeading,
	closeNewTokenDialog,
	deleteButton,
	deleteWarning,
	confirmDeleteServiceAccount,
	serviceAccountRow,
	nameTakenError,
	expectToast,
} from '@/helpers/ui/service-account';

// The detail page: the account's fields and roles, regenerating its token and
// deleting it. Token checks go straight at the API (API_DIRECT_URL), since a
// service account's token is a bearer token.

type Page = import('@playwright/test').Page;
type Owner = Awaited<ReturnType<typeof createUserWithWorkspace>>;

async function withPage(
	browser: import('@playwright/test').Browser,
	user: Owner,
	fn: (page: Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await fn(page);
	} finally {
		await context.close();
	}
}

// Order-insensitive view of a set of grants.
function sortedGrants(grants: { roleId: string; resourceId: string }[]): string[] {
	return grants.map((g) => `${g.roleId}:${g.resourceId}`).sort();
}

test.describe('service account > detail [UI]', () => {
	test('shows its ID, fields and roles', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const roleId = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [roleId],
			description: 'Deploys from GitHub Actions',
		});
		await withPage(browser, owner, async (page) => {
			await openServiceAccountDetail(page, account.id);
			await expect(copyableValue(page, account.id)).toBeVisible();
			await expect(nameInput(page)).toHaveValue(account.name, { timeout: 10_000 });
			await expect(descriptionInput(page)).toHaveValue('Deploys from GitHub Actions');
			await expect(
				page.getByRole('button', { name: 'Remove Runner: Viewer', exact: true }),
			).toBeVisible({ timeout: 10_000 });
			await expect(saveButton(page)).toBeDisabled();
		});
	});

	test('editing its roles changes what its token can do', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, owner, owner.workspaceId);
		const secretViewer = await builtInRoleId(api, owner, owner.workspaceId, 'Secret: Viewer');
		const runnerViewer = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [secretViewer],
		});
		const runnerPath = `/workspace/${owner.workspaceId}/runner/${runner.id}`;

		const before = await callWithApiToken(api, account.token, {
			clientIp: owner.clientIp,
			path: runnerPath,
		});
		expect(before.status).toBe(401);

		await withPage(browser, owner, async (page) => {
			await openServiceAccountDetail(page, account.id);
			await removeBindingRole(page, 'Secret: Viewer');
			await addBindingRole(page, 'Runner: Viewer');
			await saveButton(page).click();
			await expectToast(page, /Service account updated successfully/);
			await expect(saveButton(page)).toBeDisabled({ timeout: 10_000 });
		});

		const info = await getServiceAccountAPI(api, owner, owner.workspaceId, account.id);
		expect(info.roleBindings).toEqual([
			{ roleId: runnerViewer, resourceId: owner.workspaceId },
		]);
		const after = await callWithApiToken(api, account.token, {
			clientIp: owner.clientIp,
			path: runnerPath,
		});
		expect(after.status).toBe(200);
	});

	test('renaming keeps every grant it had, untouched', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, owner, owner.workspaceId);
		const runnerViewer = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const secretViewer = await builtInRoleId(api, owner, owner.workspaceId, 'Secret: Viewer');
		// The same role at the workspace and at one runner reads as one
		// workspace-wide row in the editor; saving must not collapse it.
		const roles = [runnerViewer, ...scopedTo(runnerViewer, [runner.id]), secretViewer];
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles,
			description: 'Old description',
		});
		const before = await getServiceAccountAPI(api, owner, owner.workspaceId, account.id);
		const renamed = randomServiceAccountName('e2e-sa-renamed');

		await withPage(browser, owner, async (page) => {
			await openServiceAccountDetail(page, account.id);
			await expect(
				page.getByRole('button', { name: 'Remove Runner: Viewer', exact: true }),
			).toBeVisible({ timeout: 10_000 });
			await nameInput(page).fill(renamed);
			await descriptionInput(page).fill('');
			await saveButton(page).click();
			await expectToast(page, /Service account updated successfully/);
		});

		const after = await getServiceAccountAPI(api, owner, owner.workspaceId, account.id);
		expect(after.name).toBe(renamed);
		expect(after.description).toBeNull();
		expect(sortedGrants(after.roleBindings)).toEqual(sortedGrants(before.roleBindings));
	});

	test('a duplicate name shows an inline error', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const roleId = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const taken = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [roleId],
		});
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [roleId],
		});
		await withPage(browser, owner, async (page) => {
			await openServiceAccountDetail(page, account.id);
			await expect(nameInput(page)).toHaveValue(account.name, { timeout: 10_000 });
			await nameInput(page).fill(taken.name);
			await saveButton(page).click();
			await expect(nameTakenError(page, taken.name)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('a role scoped to another service account shows its name', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		const runnerViewer = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const saViewer = await builtInRoleId(
			api,
			owner,
			owner.workspaceId,
			'Service Account: Viewer',
		);
		const target = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [runnerViewer],
		});
		const other = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [runnerViewer],
		});
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: scopedTo(saViewer, [target.id]),
		});
		await withPage(browser, owner, async (page) => {
			await openServiceAccountDetail(page, account.id);
			// The binding's Resources cell names the scoped service account.
			const resourceCell = page.getByText(target.name, { exact: true });
			await expect(resourceCell).toBeVisible({ timeout: 10_000 });
			await expect(page.getByText(target.id, { exact: true })).toHaveCount(0);

			// Editing the row opens its picker, which lists the workspace's
			// service accounts by name. The target is already picked.
			await page.getByRole('button', { name: /^Edit$/ }).click();
			await page.getByPlaceholder('1 selected').click();
			await expect(page.getByText(other.name, { exact: true })).toBeVisible({
				timeout: 10_000,
			});
		});
	});

	test('regenerating stops the old token and shows the new one once', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const roleId = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [roleId],
		});
		const runnersPath = `/workspace/${owner.workspaceId}/runner`;

		let newToken = '';
		await withPage(browser, owner, async (page) => {
			await openServiceAccountDetail(page, account.id);
			await regenerateToken(page, account.name);
			newToken = await readNewToken(page);
			expect(page.url()).not.toContain(newToken);
			await closeNewTokenDialog(page);
			await expect(newTokenHeading(page)).toHaveCount(0);
			await expect(page.getByText(newToken)).toHaveCount(0);
		});

		expect(newToken).not.toBe(account.token);
		const oldR = await callWithApiToken(api, account.token, {
			clientIp: owner.clientIp,
			path: runnersPath,
		});
		expect(oldR.status).toBe(401);
		const newR = await callWithApiToken(api, newToken, {
			clientIp: owner.clientIp,
			path: runnersPath,
		});
		expect(newR.status).toBe(200);
	});

	test('deleting asks first, returns to the list and stops its token', async ({
		browser,
		api,
	}) => {
		await using owner = await createUserWithWorkspace(api);
		const roleId = await builtInRoleId(api, owner, owner.workspaceId, 'Runner: Viewer');
		const account = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [roleId],
		});
		const kept = await createServiceAccountAPI(api, owner, owner.workspaceId, {
			roles: [roleId],
		});

		await withPage(browser, owner, async (page) => {
			await openServiceAccountDetail(page, account.id);
			await deleteButton(page).click();
			await expect(deleteWarning(page)).toBeVisible();
			await confirmDeleteServiceAccount(page, account.name);
			await expectUrl(page, /\/workspace\/service-accounts$/, { timeout: 10_000 });
			await expect(serviceAccountRow(page, kept.name)).toBeVisible({ timeout: 10_000 });
			await expect(serviceAccountRow(page, account.name)).toHaveCount(0);
		});

		const after = await callWithApiToken(api, account.token, {
			clientIp: owner.clientIp,
			path: `/workspace/${owner.workspaceId}/runner`,
		});
		expect(after.status).toBe(401);
	});

	test('an id that is not a UUID is a 404, with no API call', async ({ browser, api }) => {
		await using owner = await createUserWithWorkspace(api);
		await withPage(browser, owner, async (page) => {
			const calls: string[] = [];
			page.on('request', (req) => {
				if (/\/api\/workspace\/[^/]+\/service-account\//.test(req.url())) {
					calls.push(req.url());
				}
			});
			await page.goto('/workspace/service-accounts/..%2F..%2Frunner', {
				waitUntil: 'domcontentloaded',
			});
			await expect(page).toHaveTitle(/Not Found/, { timeout: 10_000 });
			expect(calls).toEqual([]);
		});
	});
});
