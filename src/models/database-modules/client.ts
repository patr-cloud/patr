import pool from '../database';
import { Client } from '../interfaces/client';

export async function getClient(clientId: string): Promise<Client | null> {
	const clients = await pool.query(
		`
		SELECT
			*
		FROM
			clients
		WHERE
			clientId = ?
		`,
		[clientId],
	);

	if (clients.length === 1) {
		const client = clients[0];
		client.redirectUris = JSON.parse(client.redirectUris);
		client.responseTypes = JSON.parse(client.responseTypes);
		client.grantTypes = JSON.parse(client.grantTypes);
		return clients[0];
	}
	return null;
}

export async function createClient(client: Client) {
	await pool.query(
		`
		INSERT INTO
			clients(clientId, clientSecret, redirectUris, responseTypes, grantTypes)
		VALUES
			(?,?,?,?,?);
		`,
		[
			client.clientId,
			client.clientSecret,
			JSON.stringify(client.redirectUris),
			JSON.stringify(client.responseTypes),
			JSON.stringify(client.grantTypes),
		],
	);

	return client;
}
