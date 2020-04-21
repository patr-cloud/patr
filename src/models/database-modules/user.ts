import { v4 } from 'uuid';

import { pool } from '../database';
import { User } from '../interfaces/user';

export async function createUser(username: string, password: string) {
    const userId = v4();
    await pool.query(
        `
        INSERT INTO
            users (userId, username, password)
        VALUES
            (?, ?, ?);
        `,
        [userId, username, password]
    );

    return {
        userId,
        username,
        password
    };
}

export async function getUserByUsername(username: string) {
    const users = await pool.query(
        `
        SELECT
            userId, username, password
        FROM
            users
        WHERE
            users.username = ?
        `,
        [username]
    );

    if (users.length === 1) {
        return users[0];
    } else {
        return null;
    }
}

export async function getUserByUserid(userId: string): Promise<User> {
    const users = await pool.query(
        `
        SELECT
            userId, username, password
        FROM
            users
        WHERE
            users.userId = ?
        `,
        [userId]
    );

    if (users.length === 1) {
        return users[0];
    } else {
        return null;
    }
}