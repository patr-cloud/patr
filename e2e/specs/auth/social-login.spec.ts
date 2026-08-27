import { test, expect, newContext, seedGithubSetupToken, sql, expectUrl } from '@/prelude';
import { expectFirstWorkspaceScreen } from '@/helpers/ui/workspace';

// /login/github and /sign-up/github are the two halves of the GitHub OAuth
// flow. Neither had any coverage, which is how both came to render their
// parent's form instead of themselves for a while (TanStack Router only shows
// child routes through an explicit <Outlet />, and the old login.tsx /
// sign-up.tsx page components had none). Every assertion below therefore
// doubles as a routing assertion: none of this text exists on the login or
// sign-up form, so a regression in the route tree fails these outright.
//
// The real GitHub token exchange is out of reach — callback.rs posts to
// hardcoded github.com URLs — but everything up to it is not. The API consumes
// the CSRF state from Redis *before* calling GitHub, and the setup token is
// just a Redis key, so both pages are fully drivable with no egress.

async function withContext(
	browser: import('@playwright/test').Browser,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
	const context = await newContext(browser);
	const page = await context.newPage();
	try {
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('social login > /login/github [UI]', () => {
	test('renders the callback page rather than the login form', async ({ browser }) => {
		await withContext(browser, async (page) => {
			// No code/state, so it will bounce to /login — but only after the
			// callback page itself has mounted and said so.
			await page.goto('/login/github', { waitUntil: 'domcontentloaded' });
			await expect(
				page.getByText(/Invalid GitHub callback — missing code or state/i),
			).toBeVisible({
				timeout: 10_000,
			});
			await expectUrl(page, /\/login$/);
		});
	});

	test('a bad CSRF state is rejected by the API and bounces to login', async ({ browser }) => {
		await withContext(browser, async (page) => {
			// State is consumed from Redis before the GitHub exchange, so this
			// exercises the real API path without any outbound request.
			await page.goto('/login/github?code=fake-code&state=not-a-real-state', {
				waitUntil: 'domcontentloaded',
			});
			await expect(page.getByText(/GitHub sign-in failed/i)).toBeVisible({ timeout: 10_000 });
			await expectUrl(page, /\/login$/);
		});
	});
});

test.describe('social login > /sign-up/github [UI]', () => {
	test('without a setup token it bounces to login', async ({ browser }) => {
		await withContext(browser, async (page) => {
			await page.goto('/sign-up/github', { waitUntil: 'domcontentloaded' });
			await expect(page.getByText(/Invalid or expired GitHub sign-in session/i)).toBeVisible({
				timeout: 10_000,
			});
			await expectUrl(page, /\/login$/);
		});
	});

	test('renders the profile form with GitHub values pre-filled', async ({ browser }) => {
		const suffix = crypto.randomUUID().replace(/-/g, '').slice(0, 12);
		const email = `e2egithub${suffix}@example.com`;
		const setupToken = await seedGithubSetupToken(email);

		await withContext(browser, async (page) => {
			await page.goto(
				`/sign-up/github?setupToken=${setupToken}&firstName=Ada&lastName=Lovelace&email=${encodeURIComponent(email)}`,
				{ waitUntil: 'domcontentloaded' },
			);
			await expect(page.getByRole('heading', { name: /Complete your profile/i })).toBeVisible(
				{
					timeout: 10_000,
				},
			);
			await expect(page.locator('#first-name')).toHaveValue('Ada');
			await expect(page.locator('#last-name')).toHaveValue('Lovelace');
		});
	});

	test('submitting the profile creates the account and signs in', async ({ browser }) => {
		const suffix = crypto.randomUUID().replace(/-/g, '').slice(0, 12);
		const email = `e2egithub${suffix}@example.com`;
		const setupToken = await seedGithubSetupToken(email);

		await withContext(browser, async (page) => {
			await page.goto(`/sign-up/github?setupToken=${setupToken}`, {
				waitUntil: 'domcontentloaded',
			});
			await page.locator('#first-name').fill('Ada');
			await page.locator('#last-name').fill('Lovelace');
			await page.getByRole('button', { name: /Create Account/i }).click();

			// A brand-new user has no workspace, so the app lands on the inline
			// create-workspace screen.
			await expectFirstWorkspaceScreen(page);

			const rows = await sql<{ first_name: string; last_name: string }>(
				'SELECT first_name, last_name FROM "user" WHERE email = $1',
				[email],
			);
			expect(rows).toHaveLength(1);
			expect(rows[0].first_name).toBe('Ada');
			expect(rows[0].last_name).toBe('Lovelace');
		});
	});
});
