// Shared, environment-bound test configuration. Keep in sync with
// `e2e/Justfile` (the values exported here mirror what the API + frontend
// processes are spawned with) and with `frontend/src/...` (where applicable).
//
// Anything in here is "what the test harness needs to know about the world it
// runs against." Per-helper magic numbers (timeouts inside a single helper,
// retry counts unique to one spec, etc.) don't belong here.

/**
 * Default wait for SolidStart hydration on a cold first navigation. Vinxi
 * dev mode routinely takes 2–5s on a cold load and the default Playwright
 * `expect` timeout (5s) loses that race. Per-helper `open*` functions all
 * `waitFor` an anchor with this timeout before returning.
 */
export const HYDRATION_TIMEOUT = 15_000;

/**
 * User-Agent string sent on every API call from the test harness. The API's
 * `user_agent_validation_layer` parses this; an empty/missing UA returns
 * 400 before any handler runs. Bumped per release of the e2e suite when
 * we need to differentiate behaviour by client version.
 */
export const USER_AGENT = 'patr-e2e/1.0';

/**
 * HS256 secret the API signs and verifies JWTs with. Mirrors the value
 * `e2e/Justfile` exports as `PATR__JWT_SECRET`. Override with `$JWT_SECRET`
 * when running against a live API instance with a different key.
 */
export const JWT_SECRET = process.env.JWT_SECRET ?? 'keyboard cat';

/**
 * Cloudflare-issued always-passes test site key, paired with the
 * always-passes test secret the Justfile sets as `PATR__CLOUDFLARE__TURNSTILE_SECRET`.
 * Sending this verbatim as `cfTurnstileToken` makes the backend's Turnstile
 * verification call return success without contacting Cloudflare.
 */
export const TURNSTILE_TOKEN = '1x00000000000000000000AA';

/**
 * In debug builds the API generates OTPs in the closed range [0, 0] (see
 * `api/src/utils/mod.rs::OTP_RANGE`), so any flow that needs to consume an
 * OTP can use this verbatim instead of reading it from the database.
 */
export const DEBUG_OTP = '000000';

/**
 * Debug builds also make the workspace-invite token deterministic (see
 * `api/src/utils/mod.rs::WORKSPACE_INVITE_DEBUG_TOKEN`), so specs can follow an
 * invite link without a mail sink to read the emailed token from.
 */
export const DEBUG_INVITE_TOKEN = '0'.repeat(64);

/** Mirrors `api/src/utils/mod.rs::MAX_PASSWORD_RESET_ATTEMPTS`. */
export const MAX_PASSWORD_RESET_ATTEMPTS = 5;
