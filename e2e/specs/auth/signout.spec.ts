import { test, expect, newContext, createUserWithWorkspace } from '@/prelude';
import { openLoginPage, fillLoginForm, submitLogin, waitForLoggedIn } from '@/helpers/ui/login';
import { signOut } from '@/helpers/ui/profile';

async function loggedInPage(
	browser: import('@playwright/test').Browser,
	api: import('@/prelude').ApiClient,
) {
	const user = await createUserWithWorkspace(api);
	const context = await newContext(browser);
	const page = await context.newPage();
	await openLoginPage(page);
	await fillLoginForm(page, { email: user.email, password: user.password });
	await submitLogin(page);
	await waitForLoggedIn(page);
	return { context, page, user };
}

test.describe('sign-out', () => {
	test('click sign-out → land on /login, authState cookie cleared', async ({ browser, api }) => {
		const { context, page } = await loggedInPage(browser, api);
		try {
			await signOut(page);
			await expect(page).toHaveURL(/\/login$/, { timeout: 10_000 });
			const cookies = await context.cookies();
			const authState = cookies.find((c) => c.name === 'authState');
			// Either cookie is gone, or has a LoggedOut payload — both are acceptable
			// "signed out" signals depending on how the SPA implements logout.
			const cleared =
				!authState ||
				!authState.value ||
				authState.value === '' ||
				decodeURIComponent(authState.value).includes('LoggedOut');
			expect(cleared).toBe(true);
		} finally {
			await context.close();
		}
	});

	test('visiting a guarded route after sign-out redirects to /login', async ({
		browser,
		api,
	}) => {
		const { context, page } = await loggedInPage(browser, api);
		try {
			await signOut(page);
			await page.goto('/profile', { waitUntil: 'domcontentloaded' });
			await expect(page).toHaveURL(/\/login/, { timeout: 10_000 });
		} finally {
			await context.close();
		}
	});
});
