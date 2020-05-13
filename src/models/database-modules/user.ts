import { v4 } from 'uuid';

import pool from '../database';
import { User } from '../interfaces/user';

export async function createUser(user: User): Promise<User> {
	user.userId = v4();
	await pool.query(
		`
        INSERT INTO
            users (userId, email, username, password)
        VALUES
            (UUID_TO_BIN(?), ?, ?, ?);
        `,
		[user.userId, user.email, user.username, user.password],
	);

	return user;
}

export async function getUserByUsername(username: string): Promise<User> {
	const users = await pool.query(
		`
        SELECT
            *
        FROM
            users
        WHERE
            users.username = ?
        `,
		[username],
	);

	if (users.length === 1) {
		return users[0];
	}
	return null;
}

export async function getUserByUserid(userId: string): Promise<User> {
	const users = await pool.query(
		`
        SELECT
            *
        FROM
            users
        WHERE
            users.userId = ?
        `,
		[userId],
	);

	if (users.length === 1) {
		return users[0];
	}
	return null;
}
