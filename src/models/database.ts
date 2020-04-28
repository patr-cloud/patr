import mysql from 'promise-mysql';
import { mysql as mysqlConfig } from '../config/config';

const poolCreationPromise = mysql.createPool(mysqlConfig);
let formedPool: mysql.Pool;

const pool = {
	async query(query: string, args: any[] = []): Promise<any> {
		if (poolCreationPromise.isResolved() === false) {
			formedPool = await poolCreationPromise;
		}
		return formedPool.query(query, args);
	},
};

export default pool;
