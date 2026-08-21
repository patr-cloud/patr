import {
	test,
	expect,
	newContext,
	createUserAccount,
	createPendingSignup,
	randomIPv4,
} from '@/prelude';
import { openSignupPage, fillSignupForm, submitSignup } from '@/helpers/ui/signup';

// Frontend sets `noValidate` on the form, so browser-level pattern/email
// validation is bypassed. Client-side validation is the JS `validateInputs`
// function: it checks `.trim()` non-empty + `validateEmail` +
// `validatePassword` + confirm-match. Anything it lets through that the
// server still rejects surfaces as a generic toast
// ("Error creating account: ...").

function newCreds(suffix = crypto.randomUUID().replace(/-/g, '').slice(0, 12)) {
	return {
		firstName: 'E2E',
		lastName: 'User',
		email: `e2euser${suffix}@example.com`,
		password: 'E2eTest!1Password',
	};
}

async function withSignupContext(
	browser: import('@playwright/test').Browser,
	fn: (page: import('@playwright/test').Page) => Promise<void>,
) {
	const context = await newContext(browser);
	const page = await context.newPage();
	try {
		await openSignupPage(page);
		await fn(page);
	} finally {
		await context.close();
	}
}

test.describe('sign-up — happy path', () => {
	test('valid credentials → navigates to /confirm-signup, email pre-filled', async ({
		browser,
	}) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, creds);
			await submitSignup(page);
			// The page strips ?email=... from the URL on mount, then renders
			// "Confirming account for <email>" instead of the email input.
			await expect(page).toHaveURL(/\/confirm-signup/, { timeout: 10_000 });
			await expect(
				page.getByText(new RegExp(`Confirming account for.*${creds.email}`)),
			).toBeVisible({ timeout: 5_000 });
		});
	});
});

test.describe('sign-up — client-side field validation', () => {
	// The form blocks submit and shows inline alerts without firing a network
	// request. We verify both: the alert text AND that no /auth/sign-up request
	// fires while we wait briefly.

	async function expectNoSignupRequest(
		page: import('@playwright/test').Page,
		action: () => Promise<void>,
	): Promise<void> {
		let fired = false;
		page.on('request', (req) => {
			if (req.url().includes('/auth/sign-up')) fired = true;
		});
		await action();
		await page.waitForTimeout(500);
		expect(fired).toBe(false);
	}

	// Note: empty `confirm-password` doesn't surface "required" — it surfaces
	// "Passwords do not match" (validateInputs only checks `!password()` for
	// required, then compares `password() !== confirmPassword()`).
	for (const field of ['first-name', 'last-name', 'email', 'password'] as const) {
		test(`empty ${field} blocks submit`, async ({ browser }) => {
			await withSignupContext(browser, async (page) => {
				const creds = newCreds();
				await fillSignupForm(page, creds);
				// Clear the field under test.
				await page.locator(`#${field}`).fill('');
				await expectNoSignupRequest(page, async () => {
					await submitSignup(page);
				});
				// An Alert appears for the field (each cleared field is required).
				await expect(page.getByText(/required/i).first()).toBeVisible();
			});
		});
	}

	test('empty confirm-password surfaces "do not match"', async ({ browser }) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, creds);
			await page.locator('#confirm-password').fill('');
			await expectNoSignupRequest(page, async () => {
				await submitSignup(page);
			});
			await expect(page.getByText(/do not match/i)).toBeVisible();
		});
	});

	test('confirm-password mismatch blocks submit', async ({ browser }) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, { ...creds, confirmPassword: creds.password + 'X' });
			await expectNoSignupRequest(page, async () => {
				await submitSignup(page);
			});
			await expect(page.getByText(/Passwords do not match/i)).toBeVisible();
		});
	});

	// frontend/src/utils/validation.ts `validatePassword` doesn't check length —
	// it only validates the four char classes. A 7-char password with all four
	// classes (e.g. `Ab1!xyz`) passes client validation; the server rejects it
	// (preprocessor `length(min = 8)`). Test the server path explicitly.
	test('password too short (7 chars) → server rejects', async ({ browser }) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, { ...creds, password: 'Ab1!xyz' });
			const respPromise = page.waitForResponse(
				(r) => r.url().includes('/auth/sign-up') && r.request().method() === 'POST',
				{ timeout: 10_000 },
			);
			await submitSignup(page);
			const resp = await respPromise;
			expect(resp.ok()).toBe(false);
		});
	});

	test('password missing uppercase blocks submit', async ({ browser }) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, { ...creds, password: 'e2etest!1password' });
			await expectNoSignupRequest(page, async () => {
				await submitSignup(page);
			});
			await expect(page.getByText(/uppercase/i)).toBeVisible();
		});
	});

	test('password missing lowercase blocks submit', async ({ browser }) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, { ...creds, password: 'E2ETEST!1PASSWORD' });
			await expectNoSignupRequest(page, async () => {
				await submitSignup(page);
			});
			await expect(page.getByText(/lowercase/i)).toBeVisible();
		});
	});

	test('password missing digit blocks submit', async ({ browser }) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, { ...creds, password: 'NoDigits!Here' });
			await expectNoSignupRequest(page, async () => {
				await submitSignup(page);
			});
			await expect(page.getByText(/digit|number/i)).toBeVisible();
		});
	});

	test('password missing special char blocks submit', async ({ browser }) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, { ...creds, password: 'E2etest11Password' });
			await expectNoSignupRequest(page, async () => {
				await submitSignup(page);
			});
			await expect(page.getByText(/special/i)).toBeVisible();
		});
	});

	// The client's `validateEmail` blocks these before any request fires, so
	// a typo never costs a round trip.
	for (const [label, email] of [
		['no local part', '@example.com'],
		['no domain', 'baduser@'],
		['contains space', 'bad user@example.com'],
		['no dot in domain', 'baduser@example'],
		['missing @', 'not-an-email'],
	] as const) {
		test(`malformed email (${label}) blocks submit`, async ({ browser }) => {
			await withSignupContext(browser, async (page) => {
				const creds = newCreds();
				await fillSignupForm(page, { ...creds, email });
				await expectNoSignupRequest(page, async () => {
					await submitSignup(page);
				});
				await expect(page.getByText(/not a valid email address/i)).toBeVisible();
			});
		});
	}

	test('whitespace-only email treated as empty', async ({ browser }) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, { ...creds, email: '   ' });
			await expectNoSignupRequest(page, async () => {
				await submitSignup(page);
			});
			await expect(page.getByText(/required/i).first()).toBeVisible();
		});
	});
});

test.describe('sign-up — server-side rejection (bypass client validation)', () => {
	// What's left for the server: availability, and the email-format rules
	// stricter than the client's shape check. The frontend dispatches a
	// generic toast for unknown errors; the explicit handler maps
	// emailUnavailable to an inline alert.

	// Wait-for-response THEN assert: the API can occasionally take 10s+ to
	// respond under sustained suite load, so polling the DOM with a fixed
	// timeout makes the assertion flaky. Waiting on the network response
	// first gives us a deterministic signal that the server has answered;
	// the inline alert is rendered synchronously after the response lands.
	async function submitAndExpectInlineError(
		page: import('@playwright/test').Page,
		matcher: RegExp,
	): Promise<void> {
		const respPromise = page.waitForResponse(
			(r) => r.url().includes('/auth/sign-up') && r.request().method() === 'POST',
			{ timeout: 30_000 },
		);
		await submitSignup(page);
		const resp = await respPromise;
		expect(resp.ok()).toBe(false);
		await expect(page.getByText(matcher)).toBeVisible();
	}

	test('email already used by active user', async ({ browser, api }) => {
		await using existing = await createUserAccount(api);
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, { ...creds, email: existing.email });
			await submitAndExpectInlineError(page, /Email is already in use/i);
		});
	});

	test('email already used by pending signup', async ({ browser, api }) => {
		const pending = await createPendingSignup(api);
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, { ...creds, email: pending.email });
			await submitAndExpectInlineError(page, /Email is already in use/i);
		});
	});

	// A local part over RFC5321's 64-char limit sails past the client's
	// email-shape check and is only caught by the server's preprocessor.
	test('email local part over 64 chars → server rejects', async ({ browser }) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, { ...creds, email: `${'a'.repeat(65)}@example.com` });
			const signupResp = page.waitForResponse(
				(r) => r.url().includes('/auth/sign-up') && r.request().method() === 'POST',
				{ timeout: 10_000 },
			);
			await submitSignup(page);
			const resp = await signupResp;
			expect(resp.ok()).toBe(false);
		});
	});

	test('double-submit fires only one network request', async ({ browser }) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, creds);
			let signupCalls = 0;
			page.on('request', (req) => {
				if (req.url().includes('/auth/sign-up') && req.method() === 'POST') {
					signupCalls++;
				}
			});
			const submit = page.locator('button[type=submit]', { hasText: /^Sign Up$/ });
			await expect(submit).toBeEnabled({ timeout: 15_000 });
			// Dispatch both clicks synchronously in the page. Racing two locator
			// clicks is flaky: the first click's navigation removes the button, and
			// the second click then retries actionability until it times out —
			// Playwright clicks retry rather than reject, so its .catch() never
			// fires. In-page dispatch guarantees both clicks land before any
			// navigation, which is also the truer test of the double-submit guard.
			await submit.evaluate((button: HTMLButtonElement) => {
				button.click();
				button.click();
			});
			await page.waitForURL(/\/confirm-signup/, { timeout: 10_000 });
			expect(signupCalls).toBe(1);
		});
	});
});

test.describe('sign-up — concurrency @racy', () => {
	// The API's create_account handler does an UPSERT on user_to_sign_up
	// (ON CONFLICT email DO UPDATE WHERE EXCLUDED.otp_expiry > NOW()).
	// Two concurrent signups for the same email typically both succeed
	// (the second overwrites the first), but the race can also leave one
	// rejected if the conditional UPDATE evaluates false at the moment the
	// second insert lands. Either outcome is acceptable; what's NOT
	// acceptable is "both rejected" — that would mean the row is unowned.
	test('two parallel contexts with the same email — at least one succeeds', async ({
		browser,
	}) => {
		const creds = newCreds();
		const run = async () => {
			const context = await newContext(browser, randomIPv4());
			const page = await context.newPage();
			try {
				await openSignupPage(page);
				await fillSignupForm(page, creds);
				const respPromise = page.waitForResponse(
					(r) => r.url().includes('/auth/sign-up') && r.request().method() === 'POST',
					{ timeout: 15_000 },
				);
				await submitSignup(page);
				const resp = await respPromise;
				return resp.ok();
			} finally {
				await context.close();
			}
		};
		const results = await Promise.all([run(), run()]);
		expect(results.some(Boolean)).toBe(true);
	});
});

test.describe('sign-up — XSS-character validation', () => {
	test('rejects script-tag firstName with inline error', async ({ browser }) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, creds);
			await page.locator('#first-name').fill('<script>x</script>');
			let fired = false;
			page.on('request', (req) => {
				if (req.url().includes('/auth/sign-up')) fired = true;
			});
			await submitSignup(page);
			await expect(
				page.getByText(/Names cannot contain <, >, &, or control characters/).first(),
			).toBeVisible({ timeout: 5_000 });
			await page.waitForTimeout(500);
			expect(fired).toBe(false);
		});
	});

	test('rejects bracket char in lastName with inline error', async ({ browser }) => {
		await withSignupContext(browser, async (page) => {
			const creds = newCreds();
			await fillSignupForm(page, creds);
			await page.locator('#last-name').fill('Doe<');
			await submitSignup(page);
			await expect(
				page.getByText(/Names cannot contain <, >, &, or control characters/).first(),
			).toBeVisible({ timeout: 5_000 });
		});
	});
});
