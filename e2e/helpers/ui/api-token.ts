import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/profile/api-tokens/new.tsx
//   .../api-tokens/$id.tsx
//   .../api-tokens/-components/{api-token-modal,regenerate-modal,token-grants-item}.tsx
//   frontend/src/components/modal/delete-resource-modal.tsx
//   frontend/src/components/chip-input.tsx

export async function openTokenList(page: Page): Promise<void> {
	await page.goto('/profile/api-tokens', { waitUntil: 'domcontentloaded' });
	// Either the empty-state heading or the table renders once hydrated.
	await page
		.locator('text=/No API Tokens Created|Token Name/i')
		.first()
		.waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
}

export async function openNewTokenPage(page: Page): Promise<void> {
	await page.goto('/profile/api-tokens/new', { waitUntil: 'domcontentloaded' });
	await page.locator('input[name="token-name"]').first().waitFor({
		state: 'visible',
		timeout: HYDRATION_TIMEOUT,
	});
}

export async function openTokenDetail(page: Page, id: string): Promise<void> {
	await page.goto(`/profile/api-tokens/${id}`, { waitUntil: 'domcontentloaded' });
	await page.locator('#token-id').first().waitFor({
		state: 'visible',
		timeout: HYDRATION_TIMEOUT,
	});
}

// Create form: input has only `name="token-name"`, no `id`.
export async function fillTokenName(page: Page, name: string): Promise<void> {
	await page.locator('input[name="token-name"]').fill(name);
}

// ChipInput: locate the inner <input>. The placeholder is only set while the
// chip list is empty (see chip-input.tsx) so we fall back to the class hook
// once any chips exist.
function chipInput(page: Page) {
	return page.locator('input.chip-input-inner').first();
}

export async function addAllowedIp(
	page: Page,
	ip: string,
	commitWith: 'Enter' | ' ' | ',' = 'Enter',
): Promise<void> {
	const input = chipInput(page);
	await input.fill(ip);
	await input.press(commitWith === 'Enter' ? 'Enter' : commitWith === ' ' ? 'Space' : ',');
}

export async function removeAllowedIp(page: Page, ip: string): Promise<void> {
	await page.getByRole('button', { name: `Remove ${ip}` }).click();
}

// NBF (Valid From) and EXP (Valid To) date inputs are named distinctly so
// their form values don't collide on serialisation.
export async function setTokenNbfInput(page: Page, isoDate: string): Promise<void> {
	await page.locator('input[name="token-nbf"]').fill(isoDate);
}

export async function setTokenExpInput(page: Page, isoDate: string): Promise<void> {
	await page.locator('input[name="token-exp"]').fill(isoDate);
}

// Workspace checkbox is labeled with the workspace name; the WorkspacePermissionItem
// renders a <Checkbox label={workspace.name}>. The Checkbox itself renders an
// sr-only <input type=checkbox> inside a <label>. Playwright refuses to click
// sr-only elements (they're absolutely-positioned and considered invisible), so
// we click the surrounding <label> directly. The sidebar shows the same
// workspace name as plain text, so we scope by hasText match against the
// workspace label.
export async function enableWorkspaceCheckbox(page: Page, workspaceName: string): Promise<void> {
	// The Checkbox renders <label class="inline-flex items-center gap-2 select-none ...">
	// containing the input and visible label span. Match by exact text.
	const escaped = workspaceName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	await page
		.locator('label')
		.filter({ hasText: new RegExp(`^${escaped}$`) })
		.first()
		.click();
}

export async function selectSuperAdminRadio(page: Page): Promise<void> {
	// Radio renders <label class="..."><input type=radio sr-only>...<span>Super Admin</span></label>.
	await page
		.locator('label')
		.filter({ hasText: /^Super Admin$/ })
		.first()
		.click();
}

export async function selectSpecificRolesRadio(page: Page): Promise<void> {
	await page
		.locator('label')
		.filter({ hasText: /^Specific Roles$/ })
		.first()
		.click();
}

export async function clickCreateToken(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^Create Token$/ }).click();
}

// Read the generated token from the CopyableField inside ApiTokenModal.
// The CopyableField renders the value as visible text.
export async function readNewTokenFromModal(page: Page): Promise<string> {
	await expect(page.getByText(/API Token Created Successfully/i)).toBeVisible({
		timeout: 15_000,
	});
	// The token text starts with "patrv1." — locate by that prefix.
	const token = await page
		.locator('text=/patrv1\\.[a-f0-9-]+\\.[a-f0-9-]+/i')
		.first()
		.innerText();
	return token.trim();
}

// Regenerate flow
export async function clickRegenerate(page: Page): Promise<void> {
	// Outlined button "REGENERATE" in the page header actions.
	await page
		.getByRole('button', { name: /^REGENERATE$/ })
		.first()
		.click();
}

export async function fillRegenerateConfirmName(page: Page, name: string): Promise<void> {
	// Modal opens with title "Regenerate API Token". Its only text input is the
	// confirm field. Scope to the modal.
	const modal = page
		.locator('text=/Regenerate API Token/i')
		.locator('xpath=ancestor::form')
		.first();
	await modal.locator('input[type="text"]').fill(name);
}

export async function submitRegenerate(page: Page): Promise<void> {
	// The submit button is the second "REGENERATE" — the one inside the modal
	// form. Wait for it to be enabled (after exact-name typed).
	const submitInModal = page
		.locator('form')
		.filter({ hasText: /Regenerate API Token/i })
		.getByRole('button', { name: /^REGENERATE$/ });
	await expect(submitInModal).toBeEnabled({ timeout: 5_000 });
	await submitInModal.click();
}

// Delete flow (DeleteModal)
export async function clickDelete(page: Page): Promise<void> {
	// Default outlined "Delete" button in the header actions. Page also shows a
	// Regenerate button right next to it; the Delete one has text "Delete".
	// The shared DeleteModal renders its trigger as a Button with text "Delete".
	await page
		.getByRole('button', { name: /^Delete$/ })
		.first()
		.click();
}

export async function fillDeleteConfirmName(page: Page, name: string): Promise<void> {
	const modal = page.locator('form').filter({ hasText: /Delete API Token/i });
	await modal.locator('input[type="text"]').fill(name);
}

export async function submitDelete(page: Page): Promise<void> {
	const submitInModal = page
		.locator('form')
		.filter({ hasText: /Delete API Token/i })
		.getByRole('button', { name: /^(Delete|Deleting\.\.\.)$/ });
	await expect(submitInModal).toBeEnabled({ timeout: 5_000 });
	await submitInModal.click();
}

export async function clickSavePermissions(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^Save Permissions$/ }).click();
}
