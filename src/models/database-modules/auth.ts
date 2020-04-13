import { v4 } from 'uuid';
import { pool } from '../database';
import { User } from '../interfaces/user';
import { Login } from '../interfaces/login';
import { Invite } from '../interfaces/invite';
import { PasswordResetRequest } from '../interfaces/password-reset-request';

export async function getUserByUsernameOrEmail(userId: string): Promise<User> {
	const rows = await pool.query(
		`
		SELECT
			*
		FROM
			users
		WHERE
			username = ? OR
			email = ?
		`,
		[userId, userId]
	);
	return rows.length === 0 ? null : rows[0];
}

export async function getUserByEmail(email: string): Promise<User> {
	const rows = await pool.query(
		`
		SELECT
			*
		FROM
			users
		WHERE
			email = ?
		`,
		[email]
	);
	return rows.length === 0 ? null : rows[0];
}

export async function isEmailAvailable(email: string): Promise<boolean> {
	const rows = await pool.query(
		`
		SELECT
			*
		FROM
			users, invites
		WHERE
			users.email = ? OR
			(
				invites.email = ? AND
				invites.tokenExpiry > ?
			)
		`,
		[email, email, Date.now()]
	);
	return rows.length === 0;
}

export async function getUserByUsername(username: string): Promise<User> {
	const rows = await pool.query(
		`
		SELECT
			*
		FROM
			users
		WHERE
			username = ?
		`,
		[username]
	);
	return rows.length === 0 ? null : rows[0];
}

export async function isUsernameAvailable(username: string): Promise<boolean> {
	const rows = await pool.query(
		`
		SELECT
			*
		FROM
			users, invites
		WHERE
			users.username = ? OR
			(
				invites.username = ? AND
				invites.tokenExpiry > ?
			)
		`,
		[username, username, Date.now()]
	);
	return rows.length === 0;
}

export async function addNewLogin(userId: string, authToken: string, tokenExpiry: number) {
	await pool.query('INSERT INTO logins (userId, authToken, authExp, lastLogin) VALUES (?, ?, ?, ?);', [userId, authToken, tokenExpiry, Date.now()]);
}

export async function getLoginForAuthToken(authToken: string): Promise<Login> {
	const rows: Login[] = await pool.query('SELECT * from logins WHERE authToken = ?;', [authToken]);
	return rows.length === 0 ? null : rows[0];
}

export async function setAuthTokenExpiry(authToken: string, expiry: number) {
	await pool.query('UPDATE logins SET authExpiry = ?, lastActivity = ? WHERE authToken = ?;', [expiry, Date.now(), authToken]);
}

export async function addEmailInvite(email: string, username: string, password: string, token: string, tokenExpiry: number) {
	await pool.query('INSERT INTO invites VALUES (?, ?, ?, ?, ?) ON DUPLICATE KEY UPDATE username = ?, password = ?, token = ?, tokenExpiry = ?;', [email, username, password, token, tokenExpiry, username, password, token, tokenExpiry]);
}

export async function getInviteForEmail(email: string): Promise<Invite> {
	const rows: Invite[] = await pool.query('SELECT * FROM invites WHERE email = ?;', [email]);
	return rows.length === 0 ? null : rows[0];
}

export async function addUser(username: string, email: string, password: string) {
	const userId = v4();
	let exists = true;
	while (exists === true) {
		const rows = await pool.query('SELECT * FROM users WHERE userId = ?', [userId]);
		exists = rows.length !== 0;
	}
	await pool.query('INSERT INTO users (userId, username, email, password) values (?, ?, ?, ?);', [userId, username, email, password]);
}

export async function deleteInvite(invite: Invite) {
	await pool.query('DELETE FROM invites WHERE email = ?;', [invite.email]);
}

export async function createPasswordResetRequest(userId: string, token: string, tokenExpiry: number) {
	await pool.query('INSERT INTO password_reset_requests VALUES (?, SHA2(?, 256), ?) ON DUPLICATE KEY UPDATE token = SHA2(?, 256), tokenExpiry = ?;', [userId, token, tokenExpiry, token, tokenExpiry]);
}

export async function getPasswordResetRequestForToken(token: string): Promise<PasswordResetRequest> {
	const rows = await pool.query('SELECT * FROM password_reset_requests WHERE token = SHA2(?, 256);', [token]);
	return rows.length === 0 ? null : rows[0];
}

export async function updateUserPassword(userId: string, password: string) {
	await pool.query('UPDATE users SET password = ? WHERE userId = ?;', [password, userId]);
}

export async function deletePasswordResetRequest(request: PasswordResetRequest) {
	await pool.query('DELETE FROM password_reset_requests WHERE userId = ?;', [request.userId]);
}

export async function getGroupsForUser(userId: string): Promise<string[]> {
	const rows = await pool.query('SELECT groupId FROM user_groups WHERE userId = ?;', [userId]);
	const groups: string[] = [];
	rows.forEach((row: any) => {
		groups.push(row.groupId);
	});
	return groups;
}
