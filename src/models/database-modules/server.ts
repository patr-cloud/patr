import { v4 } from 'uuid';
import pool from '../database';
import { Server } from '../interfaces/deployment';


export async function createServer() {
	return 1;
}

export async function getServerById(serverId: Buffer): Promise<Server> {
	const server = await pool.query(
		`
		SELECT
			*
		FROM
			servers
		WHERE
			serverId = ?
		`,
		[serverId],
	);

	if (server.length === 0) {
		return server[0];
	}
	return null;
}
