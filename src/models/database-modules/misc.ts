import { pool } from '../database';

export async function setDatabaseVersion(majorVersion: number, minorVersion: number, increments: number) {
	await pool.query(
		`
		INSERT INTO
			meta_data
		VALUES
			('majorVersion', ?)
		ON DUPLICATE KEY UPDATE
			value = ?
		`,
		[majorVersion, majorVersion]
	);
	await pool.query(
		`
		INSERT INTO
			meta_data
		VALUES
			('minorVersion', ?)
		ON DUPLICATE KEY UPDATE
			value = ?
		`,
		[minorVersion, minorVersion]
	);
	await pool.query(
		`
		INSERT INTO
			meta_data
		VALUES
			('incrementVersion', ?)
		ON DUPLICATE KEY UPDATE
			value = ?
		`,
		[increments, increments]
	);
}

export async function getDatabaseMajorVersion(): Promise<number> {
	const rows = await pool.query('SELECT * FROM meta_data WHERE metaId = ?', ['majorVersion']);
	return rows.length === 0 ? 0 : rows[0].value;
}

export async function getDatabaseMinorVersion(): Promise<number> {
	const rows = await pool.query('SELECT * FROM meta_data WHERE metaId = ?', ['minorVersion']);
	return rows.length === 0 ? 0 : rows[0].value;
}

export async function getDatabaseIncrementVersion(): Promise<number> {
	const rows = await pool.query('SELECT * FROM meta_data WHERE metaId = ?', ['incrementVersion']);
	return rows.length === 0 ? 0 : rows[0].value;
}
