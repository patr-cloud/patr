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

export async function queryUser(username: string): Promise<{
  id: string;
  username: string;
  mfaSecret: string | null;
  passwordResetAttempts: number;
} | null> {
  const { rows } = await pool.query<{
    id: string;
    username: string;
    mfa_secret: string | null;
    password_reset_attempts: number;
  }>(
    `SELECT id, username, mfa_secret, password_reset_attempts
     FROM "user" WHERE username = $1`,
    [username],
  );
  if (!rows[0]) return null;
  return {
    id: rows[0].id,
    username: rows[0].username,
    mfaSecret: rows[0].mfa_secret,
    passwordResetAttempts: rows[0].password_reset_attempts,
  };
}

// Backdate the OTP expiry on a pending signup so /auth/join treats it as
// expired without us having to actually wait 15 minutes.
export async function backdateSignupOtp(username: string, age: string): Promise<void> {
  await pool.query(
    `UPDATE user_to_sign_up
     SET otp_expiry = NOW() - $1::interval
     WHERE username = $2`,
    [age, username],
  );
}

export async function backdatePasswordResetToken(username: string, age: string): Promise<void> {
  await pool.query(
    `UPDATE "user"
     SET password_reset_token_expiry = NOW() - $1::interval
     WHERE username = $2`,
    [age, username],
  );
}

export async function exhaustPasswordResetAttempts(username: string, count: number): Promise<void> {
  await pool.query(
    `UPDATE "user"
     SET password_reset_attempts = $1
     WHERE username = $2`,
    [count, username],
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
