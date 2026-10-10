import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/_workspaced/runners/index.tsx (list)
//   frontend/src/routes/_logged-in/_workspaced/runners/new.tsx (create, then CLI setup steps)
//   frontend/src/routes/_logged-in/_workspaced/runners/$id.tsx (detail: deployments/metrics/logs)
//   frontend/src/routes/_logged-in/profile/api-tokens/-components/regenerate-modal.tsx

// ---------- List (/runners) ----------

export async function openRunnerList(page: Page): Promise<void> {
	await page.goto('/runners', { waitUntil: 'domcontentloaded' });
}

export function emptyStateHeading(page: Page) {
	return page.getByText('No Runners Added', { exact: true });
}

// "Add Runner" is a link to /runners/new (header button at >=1, empty-state CTA at 0).
export function addRunnerLink(page: Page) {
	return page.getByRole('link', { name: /Add Runner/i });
}

export function runnerRow(page: Page, name: string) {
	// List renders a mobile card grid and a desktop table (both in the DOM);
	// scope to the table so the name matches a single element at 1280 viewport.
	return page.getByRole('table').getByText(name, { exact: true });
}

// ---------- Create (/runners/new) ----------
//
// A name form that calls CreateRunner, then an inline reveal of the CLI steps
// on the same page. The token is only ever on screen, inside the reconnect
// command.

export async function openRunnerCreate(page: Page): Promise<void> {
	await page.goto('/runners/new', { waitUntil: 'domcontentloaded' });
	await page.locator('#runner-name').waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
}

export function runnerNameInput(page: Page) {
	return page.locator('#runner-name');
}

export async function fillRunnerName(page: Page, name: string): Promise<void> {
	await runnerNameInput(page).fill(name);
}

export async function submitCreateRunner(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^(Create Runner|Creating Runner\.\.\.)$/ }).click();
}

export function nameRequiredError(page: Page) {
	return page.getByText('Runner name is required.', { exact: true });
}

export function nameCharactersError(page: Page) {
	return page.getByText(
		'Runner name can only contain letters, numbers, spaces, dots (.), hyphens (-) and underscores (_).',
		{ exact: true },
	);
}

export function nameTakenError(page: Page, name: string) {
	return page.getByText(`A runner named "${name}" already exists`, { exact: true });
}

// CopyableField renders the value in a <span>, not an <input>.
export function setupCommand(page: Page, command: string) {
	return page.getByText(command, { exact: true });
}

export function tokenShownOnceAlert(page: Page) {
	return page.getByText("Copy the runner's token now. It won't be shown again.", {
		exact: true,
	});
}

// The token on its own, above the setup steps.
export function runnerTokenField(page: Page, token: string) {
	return page.getByText(token, { exact: true });
}

// The copy button next to the token.
export function copyRunnerTokenButton(page: Page, token: string) {
	return runnerTokenField(page, token).locator('xpath=..').getByRole('button', { name: 'Copy' });
}

export function reconnectCommand(page: Page, workspaceId: string, runnerId: string, token: string) {
	return setupCommand(
		page,
		`patr -w ${workspaceId} runner setup reconnect --runner-id ${runnerId} --runner-token ${token}`,
	);
}

export function goToRunnerButton(page: Page) {
	return page.getByRole('button', { name: 'Go to Runner', exact: true });
}

// The "Runners" crumb in the page head, a router link back to the list.
export function runnersBreadcrumb(page: Page) {
	return page.getByRole('heading', { name: 'Runners', exact: true }).getByRole('link');
}

// The UnsavedChangesGuard modal while the token is on screen. Buttons are
// "Stay" and "Leave".
export function leaveWithoutTokenModal(page: Page) {
	return page.getByText("Leave without the runner's token?", { exact: true });
}

// ---------- Detail (/runners/{id}) ----------

export async function openRunnerDetail(page: Page, id: string, tab?: string): Promise<void> {
	const suffix = tab === undefined ? '' : `?tab=${tab}`;
	await page.goto(`/runners/${id}${suffix}`, { waitUntil: 'domcontentloaded' });
}

// The detail page renders a StatusChip, which prints its raw lowercase status
// ("connected" / "unreachable") and relies on CSS `capitalize` for display — so
// match case-insensitively on what's actually in the DOM, the same way the list
// spec does.
// Anchored: the metrics tab (the default) has a "Last Connected" label that an
// unanchored /connected/i would match ahead of the chip.
export function statusBadge(page: Page, state: 'Online' | 'Unreachable' | 'Not set up') {
	const pattern = {
		Online: /^connected$/i,
		Unreachable: /^unreachable$/i,
		'Not set up': /^not set up$/i,
	}[state];
	return page.getByText(pattern).first();
}

export function neverConnectedCallout(page: Page) {
	return page.getByText("This runner hasn't connected yet.", { exact: true });
}

// ---------- Detail: regenerate token ----------

export function regenerateTokenButton(page: Page) {
	return page.getByRole('button', { name: 'Regenerate Token', exact: true });
}

function regenerateForm(page: Page) {
	return page.locator('form').filter({ hasText: /Regenerate Runner Token/i });
}

export function regenerateSubmit(page: Page) {
	return regenerateForm(page).getByRole('button', { name: /^REGENERATE$/ });
}

export async function regenerateToken(page: Page, runnerName: string): Promise<void> {
	await regenerateTokenButton(page).click();
	await regenerateForm(page).locator('input[type="text"]').fill(runnerName);
	await expect(regenerateSubmit(page)).toBeEnabled({ timeout: 5_000 });
	await regenerateSubmit(page).click();
}

export async function readNewRunnerToken(page: Page): Promise<string> {
	await expect(newRunnerTokenHeading(page)).toBeVisible({ timeout: 15_000 });
	const token = await newRunnerTokenDialog(page)
		.getByText(/^patr_sa_\S+$/)
		.innerText();
	return token.trim();
}

export function newRunnerTokenHeading(page: Page) {
	return page.getByText('New Runner Token', { exact: true });
}

export function newRunnerTokenDialog(page: Page) {
	return newRunnerTokenHeading(page).locator('xpath=..');
}

// The reconnect command in the dialog carries the new token.
export function newRunnerTokenCommand(
	page: Page,
	workspaceId: string,
	runnerId: string,
	token: string,
) {
	return newRunnerTokenDialog(page).getByText(
		`patr -w ${workspaceId} runner setup reconnect --runner-id ${runnerId} --runner-token ${token}`,
		{ exact: true },
	);
}

// ModalContainer's close (X) button is the first button in the dialog.
export async function closeNewRunnerTokenDialog(page: Page): Promise<void> {
	await newRunnerTokenDialog(page).getByRole('button').first().click();
}

export function deploymentsTab(page: Page) {
	return page.getByRole('button', { name: 'Deployments', exact: true });
}

export function metricsTab(page: Page) {
	return page.getByRole('button', { name: 'Metrics', exact: true });
}

export function logsTab(page: Page) {
	return page.getByRole('button', { name: 'Logs', exact: true });
}
