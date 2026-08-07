import {
	test,
	expect,
	newContext,
	createUserWithWorkspace,
	loginAs,
	createApiTokenAPI,
} from '@/prelude';
import type { ApiClient } from '@/prelude';
import { expectUrl } from '@/helpers/ui/workspace';
import { openRunnerLinkAPI, randomRunnerName, createRunnerAPI } from '@/helpers/runner-api';
import {
	openRunnerSetupInstructions,
	setupCommandField,
	openRunnerSetup,
	codeEntryHeading,
	codeEntryBoxes,
	fillSetupCode,
	linkUnavailableHeading,
	modeChoiceHeading,
	chooseNewRunner,
	chooseReconnect,
	fillRunnerName,
	submitApprove,
	nameErrorAlert,
	approvedHeading,
	rotationWarning,
	reconnectRunnerOption,
	submitReconnect,
	openRunnerList,
	runnerRow,
} from '@/helpers/ui/runner';

// Runners are no longer created from a dashboard form. The CLI opens a consent
// link and the browser approves it; `/runners/new` is just instructions now.
// Name rules (duplicate → 409, reusable after delete, cross-workspace
// uniqueness) live in the Rust API suite — api/tests/api/workspace/runner.rs.

async function withPage(
	browser: import('@playwright/test').Browser,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
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

// Open a consent link as the CLI would, returning the code the browser needs.
async function openLink(
	api: ApiClient,
	user: Awaited<ReturnType<typeof createUserWithWorkspace>>,
): Promise<string> {
	const apiToken = await createApiTokenAPI(api, user, {
		permissions: { [user.workspaceId]: { type: 'superAdmin' } },
	});
	const link = await openRunnerLinkAPI(user, user.workspaceId, apiToken.token);
	return link.userCode;
}

test.describe('runner > setup instructions [UI]', () => {
	test('/runners/new shows the CLI command instead of a create form', async ({
		browser,
		api,
	}) => {
		await using user = await createUserWithWorkspace(api);
		await withPage(browser, user, async (page) => {
			await openRunnerSetupInstructions(page);
			await expect(setupCommandField(page)).toBeVisible({ timeout: 10_000 });
			// The old create form is gone.
			await expect(page.locator('#runner-name')).toHaveCount(0);
		});
	});
});

test.describe('runner > consent link [UI]', () => {
	test('without a code, prompts for one', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withPage(browser, user, async (page) => {
			await openRunnerSetup(page);
			await expect(codeEntryHeading(page)).toBeVisible({ timeout: 10_000 });
			await expect(codeEntryBoxes(page)).toHaveCount(8);
		});
	});

	test('entering a code navigates to it', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const code = await openLink(api, user);
		await withPage(browser, user, async (page) => {
			await openRunnerSetup(page);
			await expect(codeEntryHeading(page)).toBeVisible({ timeout: 10_000 });
			await fillSetupCode(page, code);
			await page.getByRole('button', { name: /^Continue$/ }).click();
			await expectUrl(page, new RegExp(`code=${code}`), { timeout: 10_000 });
		});
	});

	test('an unknown code reports the link is unusable', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		await withPage(browser, user, async (page) => {
			// Valid alphabet, but no such link exists.
			await openRunnerSetup(page, 'ABCDEFGH');
			await expect(linkUnavailableHeading(page)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('a live code lands on the new-vs-reconnect choice', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const code = await openLink(api, user);
		await withPage(browser, user, async (page) => {
			await openRunnerSetup(page, code);
			await expect(modeChoiceHeading(page)).toBeVisible({ timeout: 10_000 });
			// Neither form is pre-rendered — the choice is explicit.
			await expect(page.locator('#runner-name')).toHaveCount(0);
		});
	});
});

test.describe('runner > approve as new [UI]', () => {
	test('approves a new runner and shows it in the list', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const code = await openLink(api, user);
		const name = randomRunnerName();
		await withPage(browser, user, async (page) => {
			await openRunnerSetup(page, code);
			await chooseNewRunner(page);
			await fillRunnerName(page, name);
			await submitApprove(page);
			await expect(approvedHeading(page)).toBeVisible({ timeout: 10_000 });

			await openRunnerList(page);
			await expect(runnerRow(page, name)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('accepts an uppercase / space / dot name', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const code = await openLink(api, user);
		const name = `My Runner.${crypto.randomUUID().slice(0, 6)}`;
		await withPage(browser, user, async (page) => {
			await openRunnerSetup(page, code);
			await chooseNewRunner(page);
			await fillRunnerName(page, name);
			await submitApprove(page);
			await expect(approvedHeading(page)).toBeVisible({ timeout: 10_000 });
		});
	});

	test('empty name: inline error, no approve call', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const code = await openLink(api, user);
		await withPage(browser, user, async (page) => {
			await openRunnerSetup(page, code);
			await chooseNewRunner(page);
			let approves = 0;
			page.on('request', (req) => {
				if (req.method() === 'POST' && /\/runner\/link\/[^/]+\/approve$/.test(req.url())) {
					approves += 1;
				}
			});
			await submitApprove(page);
			await expect(nameErrorAlert(page)).toBeVisible();
			await page.waitForTimeout(500);
			expect(approves).toBe(0);
		});
	});

	test('whitespace-only name: blocked client-side', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		const code = await openLink(api, user);
		await withPage(browser, user, async (page) => {
			await openRunnerSetup(page, code);
			await chooseNewRunner(page);
			await fillRunnerName(page, '   ');
			await submitApprove(page);
			await expect(nameErrorAlert(page)).toBeVisible();
		});
	});
});

test.describe('runner > reconnect [UI]', () => {
	test('reconnect warns about rotation and re-issues credentials', async ({ browser, api }) => {
		await using user = await createUserWithWorkspace(api);
		// An existing, never-connected runner is eligible for reconnect.
		const runner = await createRunnerAPI(api, user, user.workspaceId);
		const code = await openLink(api, user);

		await withPage(browser, user, async (page) => {
			await openRunnerSetup(page, code);
			await chooseReconnect(page);
			await expect(rotationWarning(page)).toBeVisible({ timeout: 10_000 });

			await reconnectRunnerOption(page, runner.name).click();
			await submitReconnect(page).click();
			await expect(approvedHeading(page)).toBeVisible({ timeout: 10_000 });
		});
	});
});
