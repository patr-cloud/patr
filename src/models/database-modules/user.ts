import { pool } from '../database';
import { User } from '../interfaces/user';

export async function getUserDetails(userId: string): Promise<User> {
	const rows = await pool.query('SELECT * FROM users WHERE userId = ?;', [userId]);
	return rows.length > 0 ? rows[0] : null;
}

export async function addUserToGroup(userId: string, group: string) {
	await pool.query(`INSERT INTO user_groups VALUES (?, ?);`, [userId, group]);
}

export async function updateUserDetails(userId: string, phone?: number, dob?: number, bio?: string, firstName?: string, lastName?: string, country?: string, streetAddress1?: string, streetAddress2?: string, state?: string, city?: string, pincode?: number) {
	let query = 'UPDATE users SET ';
	let firstQuery = true;
	const params = [];

	if (phone) {
		if (firstQuery !== true) {
			query += ', ';
		}
		firstQuery = false;
		query += 'phone = ?';
		params.push(phone);
	}
	if (dob) {
		if (firstQuery !== true) {
			query += ', ';
		}
		firstQuery = false;
		query += 'dob = ?';
		params.push(dob);
	}
	if (bio) {
		if (firstQuery !== true) {
			query += ', ';
		}
		firstQuery = false;
		query += 'bio = ?';
		params.push(bio);
	}
	if (firstName) {
		if (firstQuery !== true) {
			query += ', ';
		}
		firstQuery = false;
		query += 'firstName = ?';
		params.push(firstName);
	}
	if (lastName) {
		if (firstQuery !== true) {
			query += ', ';
		}
		firstQuery = false;
		query += 'lastName = ?';
		params.push(lastName);
	}
	if (country) {
		if (firstQuery !== true) {
			query += ', ';
		}
		firstQuery = false;
		query += 'country = ?';
		params.push(country);
	}
	if (streetAddress1) {
		if (firstQuery !== true) {
			query += ', ';
		}
		firstQuery = false;
		query += 'streetAddress1 = ?';
		params.push(streetAddress1);
	}
	if (streetAddress2) {
		if (firstQuery !== true) {
			query += ', ';
		}
		firstQuery = false;
		query += 'streetAddress2 = ?';
		params.push(streetAddress2);
	}
	if (state) {
		if (firstQuery !== true) {
			query += ', ';
		}
		firstQuery = false;
		query += 'state = ?';
		params.push(state);
	}
	if (city) {
		if (firstQuery !== true) {
			query += ', ';
		}
		firstQuery = false;
		query += 'city = ?';
		params.push(city);
	}
	if (pincode) {
		if (firstQuery !== true) {
			query += ', ';
		}
		firstQuery = false;
		query += 'pincode = ?';
		params.push(pincode);
	}
	query += ' WHERE userId = ?;';
	params.push(userId);

	await pool.query(query, params);
}
