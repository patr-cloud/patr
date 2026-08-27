import pg from 'pg';

const { Pool } = pg;

const pool = new Pool({
	host: '127.0.0.1',
	port: 15432,
	user: 'postgres',
	password: 'postgres',
	database: 'api',
	max: 8,
});

export async function disposeDb(): Promise<void> {
	await pool.end();
}

// Generic raw-SQL escape hatch, mirroring the thin shape of redis.ts. Specs
// run any SELECT/UPDATE/DELETE inline instead of growing per-table helpers.
// Keeps helper surface small; mostly used to assert DB invariants the UI
// can't prove on its own.
export async function sql<T extends Record<string, unknown> = Record<string, unknown>>(
	query: string,
	params: unknown[] = [],
): Promise<T[]> {
	const { rows } = await pool.query<T>(query, params);
	return rows;
}

export async function queryUser(email: string): Promise<{
	id: string;
	email: string;
	mfaSecret: string | null;
	passwordResetAttempts: number;
} | null> {
	const { rows } = await pool.query<{
		id: string;
		email: string;
		mfa_secret: string | null;
		password_reset_attempts: number;
	}>(
		`SELECT id, email, mfa_secret, password_reset_attempts
     FROM "user" WHERE email = $1::citext`,
		[email],
	);
	if (!rows[0]) return null;
	return {
		id: rows[0].id,
		email: rows[0].email,
		mfaSecret: rows[0].mfa_secret,
		passwordResetAttempts: rows[0].password_reset_attempts,
	};
}

// Backdate the OTP expiry on a pending signup so /auth/join treats it as
// expired without us having to actually wait 15 minutes.
export async function backdateSignupOtp(email: string, age: string): Promise<void> {
	await pool.query(
		`UPDATE user_to_sign_up
     SET otp_expiry = NOW() - $1::interval
     WHERE email = $2::citext`,
		[age, email],
	);
}

export async function backdatePasswordResetToken(email: string, age: string): Promise<void> {
	await pool.query(
		`UPDATE "user"
     SET password_reset_token_expiry = NOW() - $1::interval
     WHERE email = $2::citext`,
		[age, email],
	);
}

// Backdate a web_login row's token_expiry to force the SPA's refresh path
// on the next authenticated request.
export async function backdateWebLoginExpiry(loginId: string, age: string): Promise<void> {
	await pool.query(
		`UPDATE web_login
     SET token_expiry = NOW() - $1::interval
     WHERE login_id = $2`,
		[age, loginId],
	);
}

export async function deleteWebLogin(loginId: string): Promise<void> {
	await pool.query(`DELETE FROM web_login WHERE login_id = $1`, [loginId]);
}

// The machine type the deployment create form hardcodes (frontend new.tsx,
// `b3cf3771-fa39-4281-bfdf-eb2e65a061b6`). UUIDs are non-hyphenated everywhere
// in this system (the API's Uuid type rejects the hyphenated form), so this is
// the 32-hex form — used both as the API body value and the seed (Postgres
// accepts non-hyphenated uuid input). The e2e database ships with an empty
// deployment_machine_type table, so any deployment create FK-fails until this
// row exists. Seed it (idempotent).
export const DEFAULT_MACHINE_TYPE_ID = 'b3cf3771fa394281bfdfeb2e65a061b6';

export async function seedMachineType(): Promise<void> {
	await pool.query(
		`INSERT INTO deployment_machine_type (id, cpu_count, memory_count)
     VALUES ($1, 1, 1024)
     ON CONFLICT (id) DO NOTHING`,
		[DEFAULT_MACHINE_TYPE_ID],
	);
}

// Domain verification does a real public DNS TXT lookup which can't succeed in
// e2e, so mark a domain verified directly (mirrors api tests'
// mark_test_domain_verified). Required before managed URLs can be created on it.
export async function markDomainVerified(domainId: string): Promise<void> {
	await pool.query(
		`UPDATE workspace_domain SET is_verified = TRUE, last_verified = NOW() WHERE id = $1`,
		[domainId],
	);
}
