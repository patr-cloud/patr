// Common imports for specs. Re-exports the things every spec needs so a single
// `import { ... } from '@/prelude'` covers most cases. Reach for the specific
// `@/fixtures/...` or `@/helpers/...` paths only when you need something not
// re-exported here.

export { test, expect, newContext } from '@/fixtures/api';
export type { ApiClient } from '@/helpers/api';

export {
  createUserAccount,
  createUserWithWorkspace,
  createPendingSignup,
} from '@/helpers/user';
export type { User, UserHandle, PendingSignup } from '@/helpers/user';

export { RunnerHandle } from '@/helpers/runner';
export type { RunnerOpts } from '@/helpers/runner';
export { DindHandle } from '@/helpers/dind';
export type { DockerVersion } from '@/helpers/dind';

export { randomIPv4 } from '@/helpers/ip';

// Auth-flow infrastructure.
export { computeTotp } from '@/helpers/totp';
export { readMfaSetupSecret } from '@/helpers/redis';
export {
  queryUser,
  backdateSignupOtp,
  backdatePasswordResetToken,
  exhaustPasswordResetAttempts,
  backdateWebLoginExpiry,
  deleteWebLogin,
} from '@/helpers/db';

// Debug builds always emit OTP `000000` (api/src/utils/mod.rs OTP_RANGE 0..=0).
// Re-exported so specs don't have to redefine the magic constant.
export const DEBUG_OTP = '000000';
