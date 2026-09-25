import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { seedMachineType } from '@/helpers/db';
import { createContainerRepo } from '@/helpers/registry';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI } from '@/helpers/deployment-api';
import {
	createSecretAPI,
	findSecretByName,
	getSecretAPI,
	randomSecretName,
} from '@/helpers/secret-api';
import {
	openSecretDetail,
	fillSecretName,
	fillSecretValue,
	saveButton,
	deleteSecretViaModal,
} from '@/helpers/ui/secret';
import { expectToast, expectUrl } from '@/helpers/ui/workspace';

// The stored value itself (OpenBao round-trips, rotation overwriting it) is
// asserted in the Rust API suite (api/tests/api/workspace/secret.rs), which
// can read OpenBao directly. Here we cover the dashboard: edits land, a
// rotation bumps `lastUpdated` while a rename doesn't, and delete behaves.

test.beforeAll(async () => {
	await seedMachineType();
});

async function withDetail(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	id: string,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openSecretDetail(page, id);
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('secret > detail [UI]', () => {
	test('renaming saves the new name and leaves lastUpdated alone', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const secret = await createSecretAPI(
			api,
			user,
			user.workspaceId,
			randomSecretName(),
			'kept-value',
		);
		const before = await getSecretAPI(api, user, user.workspaceId, secret.id);
		const newName = randomSecretName('RENAMED');
		await withDetail(browser, user, secret.id, async (page) => {
			await expect(page.locator('#secret-name')).toHaveValue(before.name);
			await fillSecretName(page, newName);
			await saveButton(page).click();
			await expectToast(page, /Secret updated successfully/i);
			await expectUrl(page, /\/secrets$/, { timeout: 10_000 });
		});
		const after = await getSecretAPI(api, user, user.workspaceId, secret.id);
		expect(after.name).toBe(newName);
		expect(after.lastUpdated).toBe(before.lastUpdated);
	});

	test('rotating the value bumps lastUpdated', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		const secret = await createSecretAPI(api, user, user.workspaceId, name, 'old-value');
		const before = await getSecretAPI(api, user, user.workspaceId, secret.id);
		await withDetail(browser, user, secret.id, async (page) => {
			await fillSecretValue(page, 'new-value');
			await saveButton(page).click();
			await expectToast(page, /Secret updated successfully/i);
			await expectUrl(page, /\/secrets$/, { timeout: 10_000 });
		});
		const after = await getSecretAPI(api, user, user.workspaceId, secret.id);
		expect(after.name).toBe(name);
		// The API suite checks the new timestamp is later; here it just has to move.
		expect(after.lastUpdated).not.toBe(before.lastUpdated);
	});

	test('delete via modal: success toast, redirect to list, secret gone', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		const secret = await createSecretAPI(api, user, user.workspaceId, name, 'doomed');
		await withDetail(browser, user, secret.id, async (page) => {
			await deleteSecretViaModal(page, name);
			await expectToast(page, /Secret deleted successfully/i);
			await expectUrl(page, /\/secrets$/, { timeout: 10_000 });
			await expect(page.getByText(name, { exact: true })).toBeHidden();
		});
		expect(await findSecretByName(api, user, user.workspaceId, name)).toBeUndefined();
	});

	test('deleting a secret a deployment uses is refused with a toast', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const name = randomSecretName();
		const secret = await createSecretAPI(api, user, user.workspaceId, name, 'in-use');
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.id,
			environmentVariables: { API_KEY: { fromSecret: secret.id } },
		});
		await withDetail(browser, user, secret.id, async (page) => {
			await deleteSecretViaModal(page, name);
			await expectToast(page, /Secret is in use by deployment/i);
			await expectUrl(page, new RegExp(`/secrets/${secret.id}`), { timeout: 5_000 });
		});
		expect(await findSecretByName(api, user, user.workspaceId, name)).toBeDefined();
	});
});
