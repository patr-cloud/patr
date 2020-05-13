import { Router } from 'express';

const router = Router();

router.get('/callback', (req, res, next) => {
	// TODO: receive auth code and use it to get an
	// access token and an id token from the openID provider
	res.json(req.query); // JUST FOR USE WHILE TESTING
});

export default router;
