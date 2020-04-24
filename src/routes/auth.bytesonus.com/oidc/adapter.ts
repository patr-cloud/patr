import * as Redis from 'ioredis';

import { Adapter, AdapterPayload } from 'oidc-provider';
import { redis } from '../../../config/config';

const client = new Redis(redis);

function grantKeyFor(id: string) {
	return `oidc:grant:${id}`;
}

function userCodeKeyFor(userCode: string) {
	return `oidc:userCode:${userCode}`;
}

function uidKeyFor(uid: string) {
	return `oidc:uid:${uid}`;
}

class RedisAdapter implements Adapter {
	private name: string;

	constructor(name: string) {
		this.name = name;
	}

	async upsert(id: string, payload: AdapterPayload, expiresIn: number) {
		const key = this.key(id);

		const multi = client.multi();
		multi.call('JSON.SET', key, '.', JSON.stringify(payload));

		if (expiresIn) {
			multi.expire(key, expiresIn);
		}

		if (payload.grantId) {
			const grantKey = grantKeyFor(payload.grantId);
			multi.rpush(grantKey, key);
			// if you're seeing grant key lists growing out of acceptable proportions consider using LTRIM
			// here to trim the list to an appropriate length
			const ttl = await client.ttl(grantKey);
			if (expiresIn > ttl) {
				multi.expire(grantKey, expiresIn);
			}
		}

		if (payload.userCode) {
			const userCodeKey = userCodeKeyFor(payload.userCode);
			multi.set(userCodeKey, id);
			multi.expire(userCodeKey, expiresIn);
		}

		if (payload.uid) {
			const uidKey = uidKeyFor(payload.uid);
			multi.set(uidKey, id);
			multi.expire(uidKey, expiresIn);
		}

		await multi.exec();
	}

	async find(id: string) {
		const key = this.key(id);
		const data = await client.call('JSON.GET', key);
		if (!data) return undefined;
		return JSON.parse(data);
	}

	async findByUid(uid: string) {
		const id = await client.get(uidKeyFor(uid));
		return this.find(id);
	}

	async findByUserCode(userCode: string) {
		const id = await client.get(userCodeKeyFor(userCode));
		return this.find(id);
	}

	async destroy(id: string) {
		const key = this.key(id);
		await client.del(key);
	}

	// eslint-disable-next-line class-methods-use-this
	async revokeByGrantId(grantId: string) {
		const multi = client.multi();
		const tokens = await client.lrange(grantKeyFor(grantId), 0, -1);
		tokens.forEach((token: string) => multi.del(token));
		multi.del(grantKeyFor(grantId));
		await multi.exec();
	}

	async consume(id: string) {
		await client.call('JSON.SET', this.key(id), 'consumed', Math.floor(Date.now() / 1000));
	}

	key(id: string) {
		return `oidc:${this.name}:${id}`;
	}
}

export default RedisAdapter;
