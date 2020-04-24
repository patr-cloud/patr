import { v4 } from 'uuid';

import pool from '../database';
import { User } from '../interfaces/user';

export async function createUser(email:string, username: string, password: string): Promise<User> {
	const userId = v4();
	await pool.query(
		`
        INSERT INTO
            users (email, userId, username, password)
        VALUES
            (?, ?, ?, ?);
        `,
		[email, userId, username, password],
	);

	return {
		email,
		userId,
		username,
		password,
	};
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
