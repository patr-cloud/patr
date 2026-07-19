import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { seedMachineType } from '@/helpers/db';
import { expectToast, expectUrl } from '@/helpers/ui/workspace';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createContainerRepo } from '@/helpers/registry';
import {
	createDeploymentAPI,
	getDeploymentInfoAPI,
	randomDeploymentName,
} from '@/helpers/deployment-api';
import {
	openDeploymentCreate,
	openDeploymentList,
	fillDeploymentName,
	selectRegistry,
	fillImageName,
	fillImageTag,
	selectRunner,
	submitCreateDeployment,
	emptyStateHeading,
	createDeploymentLink,
	fillFirstPort,
	fillFirstEnv,
	uploadEnvFile,
	envUploadSummary,
	envUploadKeys,
	submitEnvUpload,
} from '@/helpers/ui/deployment';

// Deployment creation through the dashboard. The API contract — registry/tag/
// port/probe/config-mount round-trips, name/scale/FK validation, the TCP-enum
// gap and deployOnCreate status — lives in the Rust API suite
// (api/tests/api/workspace/deployment/mod.rs). Here we cover the create form.

test.beforeAll(async () => {
	await seedMachineType();
});

async function withCreatePage(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
): Promise<void> {
	const context = await newContext(browser, user.clientIp);
	await loginAs(context, user, { workspaceId: user.workspaceId });
	const page = await context.newPage();
	try {
		await openDeploymentCreate(page);
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('deployment > create [UI]', () => {
	test('empty list shows the empty state + create CTA', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentList(page);
			await expect(emptyStateHeading(page)).toBeVisible({ timeout: 15_000 });
			await expect(createDeploymentLink(page)).toBeVisible();
		} finally {
			await context.close();
		}
	});

	test('creates an external deployment: success toast + navigate to detail', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		await withCreatePage(browser, user, async (page) => {
			await fillDeploymentName(page, randomDeploymentName());
			await selectRegistry(page, 'Docker Hub');
			await fillImageName(page, 'traefik/whoami');
			await fillImageTag(page, 'latest');
			await selectRunner(page, runner.name);
			const respPromise = page.waitForResponse(
				(r) =>
					/\/api\/workspace\/[^/]+\/deployment$/.test(r.url()) &&
					r.request().method() === 'POST',
				{ timeout: 30_000 },
			);
			await submitCreateDeployment(page);
			expect((await respPromise).ok()).toBe(true);
			await expectToast(page, /Deployment created successfully/i);
			await expectUrl(page, /\/deployments\/[0-9a-f]{32}/, { timeout: 10_000 });
		});
	});

	test('the create form persists a port and an env var through its editors', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		let createdId = '';
		await withCreatePage(browser, user, async (page) => {
			await fillDeploymentName(page, randomDeploymentName());
			await selectRegistry(page, 'Docker Hub');
			await fillImageName(page, 'traefik/whoami');
			await fillImageTag(page, 'latest');
			await selectRunner(page, runner.name);
			// Drive the port + env editors in the form.
			await fillFirstPort(page, '8080');
			await fillFirstEnv(page, 'FOO', 'bar');
			await submitCreateDeployment(page);
			await expectToast(page, /Deployment created successfully/i);
			await expectUrl(page, /\/deployments\/[0-9a-f]{32}/, { timeout: 10_000 });
			createdId = (page.url().match(/\/deployments\/([0-9a-f]{32})/) ?? [])[1] ?? '';
		});
		// The form's editors persisted the values (no UI surface shows running
		// details, so this is verified through the API).
		expect(createdId).toMatch(/^[0-9a-f]{32}$/);
		const info = await getDeploymentInfoAPI(api, user, user.workspaceId, createdId);
		expect(info.ports).toEqual({ '8080': 'http' });
		expect(info.environmentVariables).toEqual({ FOO: 'bar' });
	});

	test('an uploaded .env file is parsed, reviewable, and persists on create', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		let createdId = '';
		// Exercises the parser through the UI: a comment, an `export ` prefix, a
		// double-quoted value with a space, and a duplicate key (last one wins).
		const dotEnv = [
			'# a comment',
			'FOO=bar',
			'export QUOTED="a b"',
			'TRAILING=keep # inline comment',
			'DUP=first',
			'DUP=second',
			'',
		].join('\n');
		await withCreatePage(browser, user, async (page) => {
			await fillDeploymentName(page, randomDeploymentName());
			await selectRegistry(page, 'Docker Hub');
			await fillImageName(page, 'traefik/whoami');
			await fillImageTag(page, 'latest');
			await selectRunner(page, runner.name);

			await uploadEnvFile(page, dotEnv);
			// 4 keys, not 5: the comment is skipped and DUP is deduped.
			await expect(envUploadSummary(page)).toContainText('4');
			await expect(envUploadKeys(page).first()).toHaveValue('FOO');
			await submitEnvUpload(page);

			// The rows landed in the env editor behind the modal.
			await expect(page.locator('input[placeholder="Enter Env Name"]').first()).toHaveValue(
				'FOO',
			);

			await submitCreateDeployment(page);
			await expectToast(page, /Deployment created successfully/i);
			await expectUrl(page, /\/deployments\/[0-9a-f]{32}/, { timeout: 10_000 });
			createdId = (page.url().match(/\/deployments\/([0-9a-f]{32})/) ?? [])[1] ?? '';
		});
		expect(createdId).toMatch(/^[0-9a-f]{32}$/);
		const info = await getDeploymentInfoAPI(api, user, user.workspaceId, createdId);
		expect(info.environmentVariables).toEqual({
			FOO: 'bar',
			QUOTED: 'a b',
			TRAILING: 'keep',
			DUP: 'second',
		});
	});

	test('the header Create button appears once there is at least one deployment', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		const repo = await createContainerRepo(api, user, user.workspaceId);
		await createDeploymentAPI(api, user, user.workspaceId, {
			repositoryId: repo.id,
			runnerId: runner.id,
		});
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentList(page);
			await expect(createDeploymentLink(page)).toBeVisible({ timeout: 15_000 });
			await expect(emptyStateHeading(page)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});
});
