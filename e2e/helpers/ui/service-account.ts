import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/_workspaced/workspace/service-accounts/index.tsx (list)
//   frontend/src/routes/_logged-in/_workspaced/workspace/service-accounts/new.tsx (create, then the token once)
//   frontend/src/routes/_logged-in/_workspaced/workspace/service-accounts/$id.tsx (detail form, regenerate, delete)
//   frontend/src/routes/_logged-in/profile/api-tokens/-components/regenerate-modal.tsx

// ---------- List (/workspace/service-accounts) ----------

export async function openServiceAccountList(page: Page, query = ''): Promise<void> {
	await page.goto(`/workspace/service-accounts${query}`, { waitUntil: 'domcontentloaded' });
	await serviceAccountsTab(page).waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
}

// The workspace header's tab, shown on every Workspace Settings page.
export function serviceAccountsTab(page: Page) {
	return page.getByRole('link', { name: 'Service Accounts', exact: true }).first();
}

export function emptyStateHeading(page: Page) {
	return page.getByText('No Service Accounts Created', { exact: true });
}

export function emptyStateDescription(page: Page) {
	return page.getByText(
		'Create a service account to give CI pipelines and other automation their own token and roles.',
		{ exact: true },
	);
}

// A link to /workspace/service-accounts/new: in the header, and as the
// empty-state CTA when there are none.
export function createServiceAccountLink(page: Page) {
	return page.getByRole('link', { name: 'Create Service Account', exact: true });
}

export function serviceAccountRow(page: Page, name: string) {
	return page.getByRole('row').filter({ has: page.getByText(name, { exact: true }) });
}

export function listHeadings(page: Page) {
	return page.getByRole('columnheader');
}

// ---------- Create (/workspace/service-accounts/new) ----------

export async function openServiceAccountCreate(page: Page): Promise<void> {
	await page.goto('/workspace/service-accounts/new', { waitUntil: 'domcontentloaded' });
	await nameInput(page).waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
}

export function nameInput(page: Page) {
	return page.locator('#service-account-name');
}

export function descriptionInput(page: Page) {
	return page.locator('#service-account-description');
}

/**
 * Adds a role through the access editor's add row: pick it in the row's role
 * dropdown (the first "Select a role" on the page), then press "Add role".
 * Scope is left at the default, the entire workspace. The options are
 * portalled to the body, so they're looked up on the page rather than under
 * the trigger.
 */
export async function selectBindingRole(page: Page, roleName: string): Promise<void> {
	await page.getByPlaceholder('Select a role').first().click();
	const escaped = roleName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	const option = page
		.locator('div.cursor-pointer')
		.filter({ hasText: new RegExp(`^${escaped}$`) })
		.first();
	await option.scrollIntoViewIfNeeded({ timeout: 10_000 });
	await option.click();
	await page.getByRole('button', { name: /^Add role$/ }).click();
}

export async function addBindingRole(page: Page, roleName: string): Promise<void> {
	await selectBindingRole(page, roleName);
}

export async function removeBindingRole(page: Page, roleName: string): Promise<void> {
	await page.getByRole('button', { name: `Remove ${roleName}`, exact: true }).click();
}

export async function submitCreateServiceAccount(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^(Create Service Account|Creating\.\.\.)$/ }).click();
}

export function nameRequiredError(page: Page) {
	return page.getByText('Service account name is required.', { exact: true });
}

export function nameCharactersError(page: Page) {
	return page.getByText(
		'Service account name can only contain letters, numbers, spaces, dots (.), hyphens (-) and underscores (_).',
		{ exact: true },
	);
}

export function nameTakenError(page: Page, name: string) {
	return page.getByText(`A service account named "${name}" already exists`, { exact: true });
}

export function descriptionCharactersError(page: Page) {
	return page.getByText('Description cannot contain <, >, &, or control characters', {
		exact: true,
	});
}

export function noRolesError(page: Page) {
	return page.getByText('Give the service account at least one role.', { exact: true });
}

export function tokenShownOnceAlert(page: Page) {
	return page.getByText("Copy this token now. It won't be shown again.", { exact: true });
}

// CopyableField renders the value in a <span>, not an <input>.
export function copyableValue(page: Page, value: string) {
	return page.getByText(value, { exact: true });
}

export function copyTokenButton(page: Page, token: string) {
	return copyableValue(page, token).locator('xpath=..').getByRole('button', { name: 'Copy' });
}

export function goToServiceAccountButton(page: Page) {
	return page.getByRole('button', { name: 'Go to Service Account', exact: true });
}

// The "Service Accounts" crumb in the page head, a router link back to the list.
export function serviceAccountsBreadcrumb(page: Page) {
	return page.getByRole('heading', { name: 'Service Accounts', exact: true }).getByRole('link');
}

export function leaveWithoutTokenModal(page: Page) {
	return page.getByText('Leave without the token?', { exact: true });
}

// ---------- Detail (/workspace/service-accounts/{id}) ----------

export async function openServiceAccountDetail(page: Page, id: string): Promise<void> {
	await page.goto(`/workspace/service-accounts/${id}`, { waitUntil: 'domcontentloaded' });
	await nameInput(page).waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
}

export function saveButton(page: Page) {
	return page.getByRole('button', { name: /^(Save Changes|Saving\.\.\.)$/ });
}

export function regenerateTokenButton(page: Page) {
	return page.getByRole('button', { name: 'Regenerate Token', exact: true });
}

export function deleteButton(page: Page) {
	return page.getByRole('button', { name: 'Delete', exact: true });
}

// The read-only access list: one row per role, then where it applies.
export function accessRow(page: Page, label: string) {
	return page.locator('li').filter({ has: page.getByText(label, { exact: true }) });
}

function regenerateForm(page: Page) {
	return page.locator('form').filter({ hasText: /Regenerate Service Account Token/ });
}

export async function regenerateToken(page: Page, name: string): Promise<void> {
	await regenerateTokenButton(page).click();
	await regenerateForm(page).locator('input[type="text"]').fill(name);
	const submit = regenerateForm(page).getByRole('button', { name: /^REGENERATE$/ });
	await expect(submit).toBeEnabled({ timeout: 5_000 });
	await submit.click();
}

export function newTokenHeading(page: Page) {
	return page.getByText('New Service Account Token', { exact: true });
}

export function newTokenDialog(page: Page) {
	return newTokenHeading(page).locator('xpath=..');
}

export async function readNewToken(page: Page): Promise<string> {
	await expect(newTokenHeading(page)).toBeVisible({ timeout: 15_000 });
	const token = await newTokenDialog(page)
		.getByText(/^patr_sa_\S+$/)
		.innerText();
	return token.trim();
}

// ModalContainer's close (X) button is the first button in the dialog.
export async function closeNewTokenDialog(page: Page): Promise<void> {
	await newTokenDialog(page).getByRole('button').first().click();
}

function deleteForm(page: Page) {
	return page.locator('form').filter({ hasText: /Delete This Service Account/ });
}

export function deleteWarning(page: Page) {
	return deleteForm(page).getByText(
		/Its token stops working immediately and its role bindings are removed\. This can't be undone\./,
	);
}

// Types the name into the open delete modal and confirms.
export async function confirmDeleteServiceAccount(page: Page, name: string): Promise<void> {
	await deleteForm(page).locator('input[type="text"]').fill(name);
	const submit = deleteForm(page).getByRole('button', { name: /^(Delete|Deleting\.\.\.)$/ });
	await expect(submit).toBeEnabled({ timeout: 5_000 });
	await submit.click();
}

export async function expectToast(page: Page, matcher: RegExp, timeout = 10_000): Promise<void> {
	await expect(page.getByText(matcher).first()).toBeVisible({ timeout });
}
