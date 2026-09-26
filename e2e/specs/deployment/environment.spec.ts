import { test, expect, newContext, createUserWithWorkspace, loginAs } from '@/prelude';
import { seedMachineType } from '@/helpers/db';
import { expectToast } from '@/helpers/ui/workspace';
import { createContainerRepo } from '@/helpers/registry';
import { createRunnerAPI } from '@/helpers/runner-api';
import { createDeploymentAPI, getDeploymentInfoAPI } from '@/helpers/deployment-api';
import { createSecretAPI, findSecretByName, randomSecretName } from '@/helpers/secret-api';
import {
	openDeploymentDetail,
	environmentTab,
	updateButton,
	fillFirstEnv,
	openConvertToSecrets,
	convertSingleEnv,
	convertEmptyState,
	convertNameInput,
	convertRowCheckbox,
	convertValueInput,
	convertNameRequiredError,
	convertSubmitButton,
	envSecretHint,
	envSecretPicker,
	valueTypeToggle,
} from '@/helpers/ui/deployment';

// Environment variables live on the deployment's "Configuration" tab. The API
// contract for env vars (replace-vs-keep, cross-workspace secret refs) is
// covered in the Rust suite (api/tests/api/workspace/deployment/mod.rs); here
// we cover the UI: the tab saves, values that look like credentials are
// flagged, and converting one turns the row into a secret reference.

test.beforeAll(async () => {
	await seedMachineType();
});

async function setup(api: import('@/prelude').ApiClient, opts: Record<string, unknown> = {}) {
	const user = await createUserWithWorkspace(api);
	const runner = await createRunnerAPI(api, user, user.workspaceId);
	const repo = await createContainerRepo(api, user, user.workspaceId);
	const dep = await createDeploymentAPI(api, user, user.workspaceId, {
		repositoryId: repo.id,
		runnerId: runner.id,
		...opts,
	});
	return { user, dep };
}

test.describe('deployment > environment [UI]', () => {
	test('the environment tab saves a new variable', async ({ browser, api }) => {
		const { user, dep } = await setup(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			await expect(environmentTab(page)).toBeVisible();

			await fillFirstEnv(page, 'LOG_LEVEL', 'debug');
			await updateButton(page).click();
			await expectToast(page, /Deployment updated successfully/i);

			const info = await getDeploymentInfoAPI(api, user, user.workspaceId, dep.id);
			expect(info.environmentVariables).toEqual({ LOG_LEVEL: 'debug' });
		} finally {
			await context.close();
		}
	});

	// A vendor token secretlint knows about is named in its own words.
	test('a recognised credential is flagged with the rule that matched', async ({
		browser,
		api,
	}) => {
		const { user, dep } = await setup(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			await fillFirstEnv(page, 'DB_URL', 'postgresql://user:hunter2@localhost:5432/app');

			// The lint is debounced and its rules are a lazily-imported chunk.
			await expect(envSecretHint(page, /connection string/i)).toBeVisible({
				timeout: 15_000,
			});
		} finally {
			await context.close();
		}
	});

	// Ordinary configuration must stay quiet, or the hint gets ignored.
	test('ordinary configuration is not flagged', async ({ browser, api }) => {
		const { user, dep } = await setup(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			await fillFirstEnv(page, 'NODE_ENV', 'production');

			// Wait past the lint debounce before asserting the absence.
			await page.waitForTimeout(2_000);
			await expect(envSecretHint(page, /looks like a secret|found /i)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	// Flipping a row to Secret only swaps the editor; its plain value stays until
	// a secret is picked. Saving then must be blocked, not store the value as
	// plaintext behind a secret picker.
	test('a row switched to Secret without picking one blocks the save', async ({
		browser,
		api,
	}) => {
		const { user, dep } = await setup(api, { environmentVariables: { DB_PASS: 'hunter2' } });
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			await valueTypeToggle(page, 'Secret').click();

			await expect(envSecretPicker(page)).toBeVisible({ timeout: 15_000 });
			await expect(page.getByText('Pick a secret', { exact: true })).toBeVisible();
			await expect(updateButton(page)).toBeDisabled();

			const info = await getDeploymentInfoAPI(api, user, user.workspaceId, dep.id);
			expect(info.environmentVariables).toEqual({ DB_PASS: 'hunter2' });
		} finally {
			await context.close();
		}
	});

	test('converting a value turns the row into a secret reference', async ({ browser, api }) => {
		const { user, dep } = await setup(api);
		const secretName = randomSecretName();
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			await fillFirstEnv(page, 'API_TOKEN', 's3cr3t-value-goes-here');

			await openConvertToSecrets(page);
			// Name opens blank — a secret's name is the user's to choose.
			await expect(convertNameInput(page).first()).toHaveValue('');
			// The value is carried over from the row.
			await expect(convertValueInput(page).first()).toHaveValue('s3cr3t-value-goes-here');

			await convertRowCheckbox(page).click();
			await convertNameInput(page).first().fill(secretName);
			await convertSubmitButton(page).click();

			// The row is now a reference, so its value input is replaced by a picker.
			await expect(envSecretPicker(page)).toBeVisible({ timeout: 15_000 });
			await expect(valueTypeToggle(page, 'Secret')).toHaveAttribute('aria-checked', 'true');

			// The secret exists in the workspace under the chosen name.
			const secret = await findSecretByName(api, user, user.workspaceId, secretName);
			expect(secret).toBeDefined();

			// Saving persists the reference rather than the literal value.
			await updateButton(page).click();
			await expectToast(page, /Deployment updated successfully/i);

			const info = await getDeploymentInfoAPI(api, user, user.workspaceId, dep.id);
			expect(info.environmentVariables).toEqual({ API_TOKEN: { fromSecret: secret!.id } });
		} finally {
			await context.close();
		}
	});

	test('a selected row cannot be converted without a name', async ({ browser, api }) => {
		const { user, dep } = await setup(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			await fillFirstEnv(page, 'API_TOKEN', 's3cr3t-value-goes-here');

			await openConvertToSecrets(page);
			await convertRowCheckbox(page).click();
			await expect(convertNameRequiredError(page)).toBeVisible();

			// Submitting with a blank name is a no-op — the modal stays put.
			await convertSubmitButton(page).click();
			await expect(convertNameRequiredError(page)).toBeVisible();
		} finally {
			await context.close();
		}
	});

	// The button stays enabled with nothing to convert: the modal explains why,
	// which beats a dead button.
	test('the modal explains itself when there is nothing to convert', async ({ browser, api }) => {
		const { user, dep } = await setup(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			await openConvertToSecrets(page);
			await expect(convertEmptyState(page)).toBeVisible();
		} finally {
			await context.close();
		}
	});

	// A key that already names a secret can't be converted again, so the row is
	// left alone rather than offered.
	test('a key matching an existing secret name is not offered for conversion', async ({
		browser,
		api,
	}) => {
		const { user, dep } = await setup(api);
		const name = randomSecretName();
		await createSecretAPI(api, user, user.workspaceId, name, 'already-stored');

		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			await fillFirstEnv(page, name, 'postgresql://user:hunter2@localhost:5432/app');

			await page.waitForTimeout(2_000);
			await expect(envSecretHint(page, /connection string|looks like a secret/i)).toHaveCount(
				0,
			);
		} finally {
			await context.close();
		}
	});

	test('the per-row Convert opens the modal with that row ticked', async ({ browser, api }) => {
		const { user, dep } = await setup(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			await fillFirstEnv(page, 'DB_URL', 'postgresql://user:hunter2@localhost:5432/app');
			await expect(envSecretHint(page, /connection string/i)).toBeVisible({
				timeout: 15_000,
			});

			await convertSingleEnv(page);
			// Pre-ticked, so the name error is already showing for that row.
			await expect(convertNameRequiredError(page)).toBeVisible();
		} finally {
			await context.close();
		}
	});
});
