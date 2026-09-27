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
	infoTab,
	updateButton,
	unsavedChangesNote,
	fillFirstEnv,
	convertEnv,
	convertModalHeading,
	convertNameInput,
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
// flagged, converting one turns its row into a secret reference, and Update
// only enables once there is something to save.

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
			const hint = envSecretHint(
				page,
				/^PostgreSQL connection string - visible to viewers\. Store as secret\?$/,
			);
			await expect(hint).toBeVisible({ timeout: 15_000 });
			// The warning colour carries it; there's no icon beside the text.
			await expect(hint.locator('xpath=..').locator('svg')).toHaveCount(0);

			// The hint and its Convert button end where the value field does.
			const field = await page
				.locator('input[placeholder="Enter Env Value"]')
				.first()
				.locator('xpath=..')
				.boundingBox();
			const convert = await page.getByRole('button', { name: /^Convert$/ }).boundingBox();
			expect(Math.abs(field!.x + field!.width - (convert!.x + convert!.width))).toBeLessThan(
				4,
			);
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
			await expect(envSecretHint(page, /visible to viewers/)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	// A real-shaped OpenAI project key is named as one, not just "possible secret".
	// Built at runtime so no key-shaped literal sits in the repo for secret
	// scanners to trip on.
	test('an OpenAI key is named as one', async ({ browser, api }) => {
		const { user, dep } = await setup(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			const key = ['sk', 'proj', 'a1'.repeat(37) + 'T3Blbk' + 'FJ' + 'b2'.repeat(37)].join(
				'-',
			);
			await fillFirstEnv(page, 'OPENAI_API_KEY', key);
			await expect(
				envSecretHint(page, /^OpenAI API token - visible to viewers\. Store as secret\?$/),
			).toBeVisible({
				timeout: 15_000,
			});
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
			// Converting is per variable, from the row's hint; there's no bulk button.
			await expect(page.getByRole('button', { name: /Convert to secrets/i })).toHaveCount(0);
			await fillFirstEnv(page, 'API_TOKEN', 's3cr3t-value-goes-here');
			await expect(
				envSecretHint(page, /^Possible secret - visible to viewers\. Convert to secret\?$/),
			).toBeVisible({
				timeout: 15_000,
			});

			await convertEnv(page);
			await expect(convertModalHeading(page)).toBeVisible();
			// Name opens blank — a secret's name is the user's to choose.
			await expect(convertNameInput(page)).toHaveValue('');
			// The value is carried over from the row.
			await expect(convertValueInput(page)).toHaveValue('s3cr3t-value-goes-here');

			await convertNameInput(page).fill(secretName);
			await convertSubmitButton(page).click();

			// The secret exists now, but the deployment isn't saved yet — both
			// the toast and the note beside Update say so.
			await expectToast(page, /^Secret created\. Update the deployment to start using it\.$/);
			await expect(unsavedChangesNote(page)).toBeVisible();

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

	test('a variable cannot be converted without a name', async ({ browser, api }) => {
		const { user, dep } = await setup(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			await fillFirstEnv(page, 'API_TOKEN', 's3cr3t-value-goes-here');
			await convertEnv(page);

			// No nagging until the user tries to convert.
			await expect(convertNameRequiredError(page)).toHaveCount(0);

			// Submitting with a blank name is a no-op — the modal stays put.
			await convertSubmitButton(page).click();
			await expect(convertNameRequiredError(page)).toBeVisible();
			await expect(convertModalHeading(page)).toBeVisible();
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
			await expect(envSecretHint(page, /visible to viewers/)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	test('the per-row Convert opens the modal for that variable', async ({ browser, api }) => {
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

			await convertEnv(page);
			await expect(page.getByText('DB_URL', { exact: true }).last()).toBeVisible();
			await expect(convertValueInput(page)).toHaveValue(
				'postgresql://user:hunter2@localhost:5432/app',
			);
		} finally {
			await context.close();
		}
	});

	// Nothing to save means nothing to click: Update enables on the first edit,
	// with a note beside it, and disables again once the edit is undone.
	test('Update only enables while there are unsaved changes', async ({ browser, api }) => {
		const { user, dep } = await setup(api, { environmentVariables: { LOG_LEVEL: 'info' } });
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			const value = page.locator('input[placeholder="Enter Env Value"]').first();
			await expect(value).toHaveValue('info', { timeout: 15_000 });
			await expect(updateButton(page)).toBeDisabled();
			await expect(unsavedChangesNote(page)).toHaveCount(0);

			await value.fill('debug');
			await expect(updateButton(page)).toBeEnabled();
			await expect(unsavedChangesNote(page)).toBeVisible();

			await value.fill('info');
			await expect(updateButton(page)).toBeDisabled();
			await expect(unsavedChangesNote(page)).toHaveCount(0);

			// Saving settles it back to disabled.
			await value.fill('debug');
			await updateButton(page).click();
			await expectToast(page, /Deployment updated successfully/i);
			await expect(updateButton(page)).toBeDisabled();
			await expect(unsavedChangesNote(page)).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	// A refresh only asks for confirmation while there's something to lose.
	test('refreshing only prompts while there are unsaved changes', async ({ browser, api }) => {
		const { user, dep } = await setup(api, { environmentVariables: { LOG_LEVEL: 'info' } });
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			const value = page.locator('input[placeholder="Enter Env Value"]').first();
			await expect(value).toHaveValue('info', { timeout: 15_000 });

			const dialogs: string[] = [];
			page.on('dialog', async (dialog) => {
				dialogs.push(dialog.type());
				await dialog.accept();
			});

			// Browsers only show the unload prompt after a user gesture, so
			// interact before each reload for the check to mean anything.
			await value.click();
			await page.reload();
			await expect(value).toHaveValue('info', { timeout: 15_000 });
			expect(dialogs).toEqual([]);

			await value.fill('debug');
			await page.reload();
			expect(dialogs).toEqual(['beforeunload']);
		} finally {
			await context.close();
		}
	});

	// Config files are part of what the container starts with, so they sit with
	// the env vars on Configuration rather than on Info.
	test('config files live on the Configuration tab', async ({ browser, api }) => {
		const { user, dep } = await setup(api);
		const context = await newContext(browser, user.clientIp);
		await loginAs(context, user, { workspaceId: user.workspaceId });
		const page = await context.newPage();
		try {
			await openDeploymentDetail(page, dep.id, 'environment');
			await expect(
				page.getByRole('heading', { name: 'Config Files', exact: true }),
			).toBeVisible({
				timeout: 15_000,
			});
			await expect(page.locator('input[name="deployment-config-filename"]')).toHaveCount(1);

			await infoTab(page).click();
			await expect(page.locator('input[name="deployment-name"]')).toBeVisible({
				timeout: 15_000,
			});
			await expect(page.locator('input[name="deployment-config-filename"]')).toHaveCount(0);
		} finally {
			await context.close();
		}
	});
});
