import { scheduleJob } from 'node-schedule';
import { pool } from './database';

// Clean up expired logins and invites
// Everyday at 4 am
scheduleJob('* * 4 * * *', async date => {
	// TODO uncomment later
	// await pool.query('DELETE FROM logins WHERE authExp < ?;', [Date.now()]);
	// await pool.query('DELETE FROM invites WHERE tokenExpiry < ?;', [Date.now()]);
});
