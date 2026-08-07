import type { Page } from '@playwright/test';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/_workspaced/runners/index.tsx (list)
//   frontend/src/routes/_logged-in/_workspaced/runners/new.tsx (CLI setup instructions)
//   frontend/src/routes/_logged-in/_workspaced/runner/setup/ (consent page)
//   frontend/src/routes/_logged-in/_workspaced/runners/$id.tsx (detail: deployments/metrics/logs)

async function waitForVisible(page: Page, selector: string): Promise<void> {
	await page.locator(selector).first().waitFor({ state: 'visible', timeout: HYDRATION_TIMEOUT });
}

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

// ---------- Setup instructions (/runners/new) ----------
//
// There is no create form any more. `/runners/new` just tells the operator to
// run the CLI; the runner is actually minted by approving a consent link.

export async function openRunnerSetupInstructions(page: Page): Promise<void> {
	await page.goto('/runners/new', { waitUntil: 'domcontentloaded' });
}

// CopyableField renders the value in a <span>, not an <input>.
export function setupCommandField(page: Page) {
	return page.getByText('patr runner setup', { exact: true });
}

// ---------- Consent page (/runner/setup) ----------

export async function openRunnerSetup(page: Page, code?: string): Promise<void> {
	const suffix = code === undefined ? '' : `?code=${code}`;
	await page.goto(`/runner/setup${suffix}`, { waitUntil: 'domcontentloaded' });
}

export function codeEntryHeading(page: Page) {
	return page.getByRole('heading', { name: 'Enter your setup code' });
}

// The 8 single-character boxes rendered by OtpInput.
export function codeEntryBoxes(page: Page) {
	return page.locator('input[name="runner-setup-code"]');
}

export async function fillSetupCode(page: Page, code: string): Promise<void> {
	await codeEntryBoxes(page).first().fill(code[0]);
	for (let i = 1; i < code.length; i += 1) {
		await codeEntryBoxes(page).nth(i).fill(code[i]);
	}
}

export function linkUnavailableHeading(page: Page) {
	return page.getByRole('heading', { name: /This link can't be used/i });
}

// ---------- Consent page: mode choice ----------

export function modeChoiceHeading(page: Page) {
	return page.getByRole('heading', { name: 'What would you like to do?' });
}

export function newRunnerChoice(page: Page) {
	return page.getByRole('button', { name: /New runner/ });
}

export function reconnectChoice(page: Page) {
	return page.getByRole('button', { name: /Reconnect/ });
}

export async function chooseNewRunner(page: Page): Promise<void> {
	await newRunnerChoice(page).click();
	await waitForVisible(page, '#runner-name');
}

export async function chooseReconnect(page: Page): Promise<void> {
	await reconnectChoice(page).click();
}

// ---------- Consent page: approve as a new runner ----------

export async function fillRunnerName(page: Page, name: string): Promise<void> {
	await page.locator('#runner-name').fill(name);
}

export async function submitApprove(page: Page): Promise<void> {
	await page.getByRole('button', { name: /^(Approve|Approving\.\.\.)$/ }).click();
}

export function nameErrorAlert(page: Page) {
	return page.getByText('Runner name is required.', { exact: true });
}

export function approvedHeading(page: Page) {
	return page.getByRole('heading', { name: 'Runner approved' });
}

// ---------- Consent page: reconnect ----------

export function rotationWarning(page: Page) {
	return page.getByText(/Reconnecting rotates this runner's credentials/i);
}

// Each candidate runner is a role="radio" button; connected ones are disabled.
export function reconnectRunnerOption(page: Page, name: string) {
	return page.getByRole('radio', {
		name: new RegExp(name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')),
	});
}

export function submitReconnect(page: Page) {
	return page.getByRole('button', { name: /^(Reconnect|Reconnecting\.\.\.)$/ });
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
export function statusBadge(page: Page, state: 'Online' | 'Unreachable') {
	const pattern = state === 'Online' ? /^connected$/i : /^unreachable$/i;
	return page.getByText(pattern).first();
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
