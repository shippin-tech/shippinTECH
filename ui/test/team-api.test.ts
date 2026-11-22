import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { registryClient } from '../src/lib/contract/registry';
import {
	createPerson,
	createTeam,
	draftFromPerson,
	loadDirectory,
	type PersonDraft,
	removeAccount,
	setMembership,
	toPerson,
	updatePerson
} from '../src/lib/shippin/people/team';

type Call = { url: string; method: string; body?: unknown; tag: string | null };
let calls: Call[];
let replies: { status: number; body?: unknown; etag?: string }[];
let store: Map<string, string>;
const rawPerson = {
	id: 'p',
	'@type': 'User',
	name: 'ada',
	domainId: 'd',
	description: 'Ada',
	emailAddress: 'ada@test.invalid',
	roles: { '@type': 'User' },
	credentials: { '7': { '@type': 'Password' } },
	memberGroupIds: { existing: true },
	quotas: { maxDiskQuota: 1073741824 }
};
const person = () => toPerson(rawPerson)!;
const draft: PersonDraft = {
	name: ' ADA ',
	domainId: 'd',
	fullName: 'Ada',
	password: 'synthetic password',
	role: 'user',
	limitGb: '1',
	teamIds: ['existing']
};

beforeEach(() => {
	calls = [];
	replies = [];
	store = new Map([['shippin-basic', btoa('test:synthetic')]]);
	vi.stubGlobal('sessionStorage', {
		getItem: (key: string) => store.get(key) ?? null,
		removeItem: (key: string) => void store.delete(key)
	});
	vi.stubGlobal(
		'fetch',
		vi.fn(async (url: string, init?: RequestInit) => {
			calls.push({
				url,
				method: init?.method ?? 'GET',
				body: init?.body ? JSON.parse(String(init.body)) : undefined,
				tag: new Headers(init?.headers).get('If-Match')
			});
			const reply = replies.shift();
			if (!reply) throw new Error('Unscripted request');
			return new Response(reply.status === 204 ? null : JSON.stringify(reply.body ?? null), {
				status: reply.status,
				headers: reply.etag ? { ETag: reply.etag } : undefined
			});
		})
	);
});
afterEach(() => vi.unstubAllGlobals());

describe('People and teams owned API', () => {
	it('accepts id-only creation acknowledgements for people and groups', async () => {
		replies.push({ status: 201, body: { id: 'p' } }, { status: 201, body: { id: 'g' } });
		await createPerson(draft);
		await createTeam({ name: ' SALES ', domainId: 'd', description: ' Sales team ' });
		expect(calls.map((call) => [call.url, call.method])).toEqual([
			['/api/v1/registry/Account', 'POST'],
			['/api/v1/registry/Account', 'POST']
		]);
		expect(calls[0].body).toMatchObject({
			'@type': 'User',
			name: 'ada',
			credentials: { '0': { '@type': 'Password', secret: draft.password } }
		});
		expect(calls[1].body).toEqual({
			'@type': 'Group',
			name: 'sales',
			domainId: 'd',
			description: 'Sales team'
		});
	});
	it('does not report successful creation when the acknowledgement has no id', async () => {
		replies.push({ status: 201, body: {} });
		await expect(createPerson(draft)).rejects.toThrow(/unexpected/i);
		expect(calls).toHaveLength(1);
	});
	it('resets only the existing password field, preserving MFA and other credentials', async () => {
		replies.push({ status: 200, body: rawPerson });
		expect(
			await updatePerson(person(), { ...draftFromPerson(person()), password: 'new synthetic password' })
		).toBe(true);
		expect(calls[0]).toMatchObject({
			url: '/api/v1/registry/Account/p',
			method: 'PATCH',
			body: { 'credentials/7/secret': 'new synthetic password' }
		});
	});
	it('adds a password credential when none exists without replacing the map', async () => {
		const current = { ...person(), passwordKey: null };
		replies.push({ status: 200, body: rawPerson });
		await updatePerson(current, { ...draftFromPerson(current), password: 'new synthetic password' });
		expect(calls[0].body).toEqual({
			'credentials/0': { '@type': 'Password', secret: 'new synthetic password' }
		});
	});
	it('preserves untouched roles, quotas and memberships and avoids no-op requests', async () => {
		const current = person();
		expect(await updatePerson(current, draftFromPerson(current))).toBe(false);
		expect(calls).toHaveLength(0);
		replies.push({ status: 200, body: rawPerson });
		await updatePerson(current, { ...draftFromPerson(current), fullName: '' });
		expect(calls[0].body).toEqual({ description: null });
	});
	it('adds and removes memberships while retaining other teams', async () => {
		replies.push({ status: 200, body: rawPerson }, { status: 200, body: rawPerson });
		await setMembership(person(), 'second', true);
		await setMembership({ ...person(), teamIds: ['existing', 'second'] }, 'second', false);
		expect(calls.map((call) => call.body)).toEqual([
			{ memberGroupIds: { existing: true, second: true } },
			{ memberGroupIds: { existing: true } }
		]);
	});
	it('loads every page of accounts and domains and retains person metadata', async () => {
		const paths: string[] = [];
		vi.stubGlobal(
			'fetch',
			vi.fn(async (url: string) => {
				paths.push(url);
				const query = new URL(url, 'http://test');
				const second = query.searchParams.get('position') === '1';
				const items = query.pathname.endsWith('/Account')
					? second
						? [{ id: 'g', '@type': 'Group', name: 'sales' }]
						: [rawPerson]
					: second
						? [{ id: 'a', name: 'a.test' }]
						: [{ id: 'z', name: 'z.test' }];
				return Response.json({ items, position: second ? 1 : 0, total: 2 });
			})
		);
		const directory = await loadDirectory();
		expect(directory.people).toEqual([person()]);
		expect(directory.teams.map((team) => team.id)).toEqual(['g']);
		expect(directory.domains.map((domain) => domain.name)).toEqual(['a.test', 'z.test']);
		expect(paths).toHaveLength(4);
		expect(paths.every((path) => path.startsWith('/api/v1/registry/'))).toBe(true);
	});
	it('keeps duplicate-name feedback and propagates refused deletion and expired sessions', async () => {
		replies.push(
			{ status: 409, body: { type: 'primaryKeyViolation', title: 'Conflict', status: 409 } },
			{ status: 403, body: { type: 'forbidden', title: 'Not allowed', status: 403 } },
			{ status: 401, body: { type: 'unauthorized', title: 'Expired', status: 401 } }
		);
		await expect(createPerson(draft)).rejects.toThrow('That name is already taken.');
		await expect(removeAccount('p')).rejects.toThrow('Not allowed');
		await expect(removeAccount('p')).rejects.toThrow('sign-in required');
		expect(store.has('shippin-basic')).toBe(false);
	});
	it('validates credential paths and values before sending anything', async () => {
		const c = registryClient({ authorization: () => 'test' });
		for (const patch of [
			{ 'credentials/7/secret': 42 },
			{ 'credentials/7/unknown': 'x' },
			{ 'credentials/__proto__': { '@type': 'Password', secret: 'synthetic' } },
			{ credentials: {}, 'credentials/7/secret': 'synthetic' },
			{ 'credentials/7': { '@type': 'Password', secret: 'synthetic' }, 'credentials/7/secret': 'synthetic' }
		])
			expect((await c.updatePerson('p', patch)).ok).toBe(false);
		expect(calls).toHaveLength(0);
	});
	it('honours an existing ETag and reports stale edits', async () => {
		const c = registryClient({ authorization: () => 'test' });
		replies.push(
			{ status: 200, body: rawPerson, etag: '"v1"' },
			{ status: 412, body: { type: 'stale', title: 'Changed', status: 412 } }
		);
		await c.get('Account', 'tagged');
		const result = await c.updatePerson('tagged', { description: 'New name' });
		expect(result.ok).toBe(false);
		expect(calls[1].tag).toBe('"v1"');
	});
});
