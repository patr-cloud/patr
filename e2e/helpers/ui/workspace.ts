import type { BrowserContext, Page } from '@playwright/test';
import { expect } from '@playwright/test';
import { DASHBOARD_URL } from '@/helpers/urls';
import { HYDRATION_TIMEOUT } from '@/helpers/config';

// Frontend reference:
//   frontend/src/routes/_logged-in/_non-workspaced/onboard.tsx
//   frontend/src/routes/_logged-in/_workspaced/workspace_/new.tsx
//   frontend/src/routes/_logged-in/_workspaced/workspace/index.tsx
//   frontend/src/components/sidebar/workspace-switcher.tsx
//
// All three forms share `#workspace-name`. Submit buttons differ by text.

async function waitForVisible(page: Page, selector: string): Promise<void> {
  await page.locator(selector).first().waitFor({
    state: 'visible',
    timeout: HYDRATION_TIMEOUT,
  });
}

// ---------- Onboard (/onboard) ----------

export async function openOnboardPage(page: Page): Promise<void> {
  await page.goto('/onboard', { waitUntil: 'domcontentloaded' });
  await waitForVisible(page, '#workspace-name');
}

export async function fillOnboardName(page: Page, name: string): Promise<void> {
  await page.locator('#workspace-name').fill(name);
}

export function onboardSubmitButton(page: Page) {
  return page.getByRole('button', { name: /^(Create Workspace|Creating\.\.\.)$/ });
}

export async function submitOnboard(page: Page): Promise<void> {
  await onboardSubmitButton(page).click();
}

// ---------- /workspace/new ----------

export async function openCreateWorkspacePage(page: Page): Promise<void> {
  await page.goto('/workspace/new', { waitUntil: 'domcontentloaded' });
  await waitForVisible(page, '#workspace-name');
}

export async function submitCreateWorkspace(page: Page): Promise<void> {
  await page
    .getByRole('button', {
      name: /^(Create Workspace|Creating Workspace\.\.\.)$/,
    })
    .click();
}

// ---------- /workspace (settings) ----------

export async function openWorkspaceSettings(page: Page): Promise<void> {
  await page.goto('/workspace', { waitUntil: 'domcontentloaded' });
  await waitForVisible(page, '#workspace-name');
}

export async function setWorkspaceName(page: Page, name: string): Promise<void> {
  await page.locator('#workspace-name').fill(name);
}

export async function clickUpdate(page: Page): Promise<void> {
  await page.getByRole('button', { name: /^Update$/ }).click();
}

export async function expectUpdateDisabled(page: Page): Promise<void> {
  await expect(page.getByRole('button', { name: /^Update$/ })).toBeDisabled();
}

export async function expectUpdateEnabled(page: Page): Promise<void> {
  await expect(page.getByRole('button', { name: /^Update$/ })).toBeEnabled();
}

// ---------- Sidebar switcher ----------

// Trigger is the row showing the current workspace name (or "Select A
// Workspace"). It's not a button — it's a clickable div with a sibling
// gear icon. Locate via the workspace-name text inside the sidebar.
export async function openWorkspaceSwitcher(page: Page): Promise<void> {
  // The trigger has cursor-pointer + an Initials avatar; the simplest
  // reliable hook is the "Workspaces" panel heading being absent before /
  // present after the click. Clicking the visible workspace label opens it.
  const trigger = page
    .locator('div.cursor-pointer:has(p.text-sm.text-white)')
    .filter({ hasText: /.+/ })
    .first();
  await trigger.click();
  await expect(page.getByText('Workspaces', { exact: true })).toBeVisible({
    timeout: 5_000,
  });
}

export async function closeWorkspaceSwitcher(page: Page): Promise<void> {
  // Click an arbitrary safe spot outside the panel (top of viewport).
  await page.mouse.click(10, 10);
  await expect(page.getByText('Workspaces', { exact: true })).toBeHidden({
    timeout: 5_000,
  });
}

export async function clickSwitcherWorkspace(page: Page, name: string): Promise<void> {
  // Inside the open panel, each workspace is a Button containing the name.
  await page.getByRole('button', { name }).click();
}

export async function clickSwitcherCreateNew(page: Page): Promise<void> {
  await page.getByRole('link', { name: /^CREATE WORKSPACE$/ }).click();
}

// Reads the current selected workspace name from the sidebar trigger row.
export async function getActiveSwitcherWorkspaceName(page: Page): Promise<string> {
  // The trigger row's <p> shows either the workspace name or "Select A
  // Workspace". Return whatever text is there, trimmed.
  const text = await page
    .locator('div.cursor-pointer:has(p.text-sm.text-white)')
    .first()
    .locator('p.text-sm.text-white')
    .first()
    .innerText();
  return text.trim();
}

// Returns the visible workspace names inside the open switcher panel.
// Assumes the panel is already open.
export async function listSwitcherWorkspaceNames(page: Page): Promise<string[]> {
  const panel = page
    .locator('div')
    .filter({ has: page.getByText('Workspaces', { exact: true }) })
    .last();
  const names = await panel.locator('button p.text-sm.text-white').allInnerTexts();
  return names.map((n) => n.trim()).filter(Boolean);
}

// ---------- URL polling (workaround for Playwright + Vinxi quirk) ----------

// expect(page).toHaveURL() hangs indefinitely against the Vinxi dev server
// after createEffect-based client-side redirects (e.g. _workspaced → /onboard
// for zero-workspace users). page.url() reads fine; the bug is in Playwright's
// internal waitForURL signal handling under HMR. Roll our own poll on Node's
// setTimeout instead.
export async function expectUrl(
  page: Page,
  pattern: RegExp,
  opts: { timeout?: number; interval?: number } = {},
): Promise<void> {
  const timeout = opts.timeout ?? 10_000;
  const interval = opts.interval ?? 100;
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (pattern.test(page.url())) return;
    await new Promise((r) => setTimeout(r, interval));
  }
  throw new Error(`expectUrl(${pattern}) timed out after ${timeout}ms; last url=${page.url()}`);
}

export async function expectUrlNot(
  page: Page,
  pattern: RegExp,
  opts: { timeout?: number; interval?: number } = {},
): Promise<void> {
  const timeout = opts.timeout ?? 10_000;
  const interval = opts.interval ?? 100;
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (!pattern.test(page.url())) return;
    await new Promise((r) => setTimeout(r, interval));
  }
  throw new Error(`expectUrlNot(${pattern}) still matched after ${timeout}ms; url=${page.url()}`);
}

// ---------- Generic toast / inline alert ----------

export async function expectToast(page: Page, matcher: RegExp, timeout = 10_000): Promise<void> {
  await expect(page.getByText(matcher).first()).toBeVisible({ timeout });
}

// ---------- Cookies ----------

export async function getLastWorkspaceIdCookie(context: BrowserContext): Promise<string | null> {
  const cookies = await context.cookies(DASHBOARD_URL);
  const c = cookies.find((c) => c.name === 'lastWorkspaceId');
  if (!c) return null;
  // Cookie value is JSON-stringified (and may be URI-encoded if the frontend's
  // cookieStorage wrote it through document.cookie). Try a few forms.
  try {
    return JSON.parse(c.value) as string;
  } catch {
    try {
      return JSON.parse(decodeURIComponent(c.value)) as string;
    } catch {
      return c.value;
    }
  }
}

export async function setLastWorkspaceIdCookie(context: BrowserContext, id: string): Promise<void> {
  await context.addCookies([
    {
      name: 'lastWorkspaceId',
      value: JSON.stringify(id),
      url: DASHBOARD_URL,
      sameSite: 'Strict',
    },
  ]);
}
