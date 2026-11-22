import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { jmap } from '../src/lib/adapter/jmap';
import { loadRegistryPage, writeRegistry } from '../src/lib/adapter/portal';
import { facadeName, servedByFacade } from '../src/lib/adapter/registry-facade';

type Call = { url: string; method: string; body?: unknown; auth?: string; ifMatch?: string };
let calls: Call[];
let answers: { status: number; body?: unknown; etag?: string }[];
let store: Map<string, string>;

beforeEach(() => {
	calls = [];
	answers = [];
	store = new Map([
		['shippin-basic', 'dGVzdDp0ZXN0'],
		['shippin-account', JSON.stringify({ username: 'admin', accountId: 'a' })]
	]);
	vi.stubGlobal('sessionStorage', {
		getItem: (k: string) => store.get(k) ?? null,
		setItem: (k: string, v: string) => void store.set(k, v),
		removeItem: (k: string) => void store.delete(k)
	});
	vi.stubGlobal(
		'fetch',
		vi.fn(async (url: string, init?: RequestInit) => {
			const headers = new Headers(init?.headers);
			calls.push({
				url: String(url),
				method: init?.method ?? 'GET',
				body: init?.body ? JSON.parse(String(init.body)) : undefined,
				auth: headers.get('Authorization') ?? undefined,
				ifMatch: headers.get('If-Match') ?? undefined
			});
			const answer = answers.shift();
			if (!answer) throw new Error(`no answer scripted for ${url}`);
			return new Response(answer.status === 204 ? null : JSON.stringify(answer.body ?? null), {
				status: answer.status,
				headers: answer.etag ? { ETag: answer.etag } : undefined
			});
		})
	);
});
afterEach(() => vi.unstubAllGlobals());

const domain = (id: string, name: string) => ({ id, name, allowRelaying: false });

describe('which objects go through the facade', () => {
	it('names a registry object by dropping the x: prefix', () => {
		expect(facadeName('x:Domain')).toBe('Domain');
		expect(facadeName('Mailbox')).toBeNull();
	});

	it('routes every private object through the owned API', async () => {
		expect(await servedByFacade('x:Domain')).toBe(true);
		expect(await servedByFacade('x:NoSuchObject')).toBe(true);
		expect(await servedByFacade('Mailbox')).toBe(false);
	});

	it('refuses unknown private objects without falling back to JMAP', async () => {
		await expect(loadRegistryPage('x:NoSuchObject')).rejects.toThrow();
		await expect(writeRegistry('x:NoSuchObject', { destroy: ['a'] }, 'a')).rejects.toThrow();
		expect(calls).toHaveLength(0);
	});

	it('refuses private methods before sending even a mixed JMAP batch', async () => {
		await expect(
			jmap(
				[],
				[
					['Mailbox/get', { accountId: 'a' }, 'm'],
					['x:Account/get', { accountId: 'a' }, 'a']
				]
			)
		).rejects.toThrow('Administrative operations must use the shippinTECH API');
		expect(calls).toHaveLength(0);
	});

	it('keeps standard JMAP mail operations available', async () => {
		const response = ['Mailbox/get', { list: [{ id: 'inbox' }] }, 'm'];
		answers.push({ status: 200, body: { methodResponses: [response] } });
		expect(await jmap([], [['Mailbox/get', { accountId: 'a' }, 'm']])).toEqual([response]);
		expect(calls).toHaveLength(1);
		expect(calls[0].body).toMatchObject({ methodCalls: [['Mailbox/get', { accountId: 'a' }, 'm']] });
	});
});

describe('listing', () => {
	it('asks the facade for one page with sort, filter and position', async () => {
		answers.push({ status: 200, body: { items: [domain('b', 'b.test')], position: 25, total: 60 } });
		const page = await loadRegistryPage('x:Domain', {
			filterText: 'b',
			sortBy: 'name',
			sortAscending: false,
			limit: 25,
			position: 25
		});
		expect(calls).toHaveLength(1);
		expect(calls[0].method).toBe('GET');
		const url = new URL(calls[0].url, 'http://x');
		expect(url.pathname).toBe('/api/v1/registry/Domain');
		expect(url.searchParams.get('sort')).toBe('-name');
		expect(url.searchParams.get('limit')).toBe('25');
		expect(url.searchParams.get('position')).toBe('25');
		expect(JSON.parse(url.searchParams.get('filter') ?? '{}')).toEqual({ text: 'b' });
		expect(calls[0].auth).toBe('Basic dGVzdDp0ZXN0');
		expect(page.items.map((r) => r.name)).toEqual(['b.test']);
		expect(page.total).toBe(60);
		expect(page.hasPrev).toBe(true);
		expect(page.hasNext).toBe(true);
	});

	it('has no next page when the page reaches the total', async () => {
		answers.push({ status: 200, body: { items: [domain('b', 'b.test')], position: 0, total: 1 } });
		const page = await loadRegistryPage('x:Domain', { limit: 25 });
		expect(page.hasNext).toBe(false);
		expect(page.hasPrev).toBe(false);
	});

	it('reads a singleton by its fixed id, without an id in the path', async () => {
		answers.push({ status: 200, body: { id: 'singleton', locale: 'en-US' } });
		const page = await loadRegistryPage('x:AccountSettings', { ids: ['singleton'] });
		expect(new URL(calls[0].url, 'http://x').pathname).toBe('/api/v1/registry/AccountSettings');
		expect(page.items).toEqual([{ id: 'singleton', locale: 'en-US' }]);
	});

	it('fetches listed ids one by one and skips one that is gone', async () => {
		answers.push({ status: 200, body: domain('a', 'a.test') });
		answers.push({ status: 404, body: { type: 'notFound', title: 'No such record', status: 404 } });
		const page = await loadRegistryPage('x:Domain', { ids: ['a', 'zz'] });
		expect(page.items.map((r) => r.id)).toEqual(['a']);
		expect(calls.map((c) => new URL(c.url, 'http://x').pathname)).toEqual([
			'/api/v1/registry/Domain/a',
			'/api/v1/registry/Domain/zz'
		]);
	});

	it('shows only what the schema names', async () => {
		answers.push({ status: 200, body: { items: [{ ...domain('b', 'b.test'), leaked: 1 }], position: 0 } });
		const page = await loadRegistryPage('x:Domain', {});
		expect(page.items[0].leaked).toBeUndefined();
	});

	it('refuses a record that breaks the contract, and says so', async () => {
		answers.push({ status: 200, body: { items: [{ id: 'b', allowRelaying: 'yes' }], position: 0 } });
		await expect(loadRegistryPage('x:Domain', {})).rejects.toThrow(/unexpected/i);
	});

	it('ends the session on 401 like the JMAP path', async () => {
		answers.push({ status: 401, body: { type: 'about:blank', title: 'Unauthorized', status: 401 } });
		await expect(loadRegistryPage('x:Domain', {})).rejects.toThrow('sign-in required');
		expect(store.has('shippin-basic')).toBe(false);
	});

	it('does not call the server when signed out', async () => {
		store.clear();
		await expect(loadRegistryPage('x:Domain', {})).rejects.toThrow('sign-in required');
		expect(calls).toHaveLength(0);
	});
});

describe('writing', () => {
	it('creates with POST and returns the new id', async () => {
		answers.push({ status: 201, body: domain('new1', 'n.test') });
		const id = await writeRegistry('x:Domain', { create: { tmp: { name: 'n.test' } } }, 'tmp');
		expect(calls[0]).toMatchObject({ method: 'POST', body: { name: 'n.test' } });
		expect(new URL(calls[0].url, 'http://x').pathname).toBe('/api/v1/registry/Domain');
		expect(id).toBe('new1');
	});

	it('updates with PATCH to the record', async () => {
		answers.push({ status: 200, body: { ...domain('b', 'b.test'), description: 'x' } });
		const id = await writeRegistry('x:Domain', { update: { b: { description: 'x' } } }, 'b');
		expect(calls[0]).toMatchObject({ method: 'PATCH', body: { description: 'x' } });
		expect(new URL(calls[0].url, 'http://x').pathname).toBe('/api/v1/registry/Domain/b');
		expect(id).toBe('b');
	});

	it('deletes with DELETE', async () => {
		answers.push({ status: 204 });
		await writeRegistry('x:Domain', { destroy: ['b'] }, 'b');
		expect(calls[0].method).toBe('DELETE');
		expect(new URL(calls[0].url, 'http://x').pathname).toBe('/api/v1/registry/Domain/b');
	});

	it('keeps the message for a record that is still referenced', async () => {
		answers.push({ status: 409, body: { type: 'objectIsLinked', title: 'Linked', status: 409 } });
		await expect(writeRegistry('x:Domain', { destroy: ['b'] }, 'b')).rejects.toThrow(
			'This record is still referenced by other objects'
		);
	});

	it('names the fields the server refused', async () => {
		answers.push({
			status: 422,
			body: { type: 'invalidProperties', title: 'Invalid', status: 422, fields: { name: 'Taken' } }
		});
		await expect(writeRegistry('x:Domain', { create: { t: { name: 'a.test' } } }, 't')).rejects.toThrow(
			/name: Taken/
		);
	});

	it('does not send a value the contract refuses', async () => {
		await expect(
			// biome-ignore lint/suspicious/noExplicitAny: a form could send anything
			writeRegistry('x:Domain', { create: { t: { name: 42 } as any } }, 't')
		).rejects.toThrow(/not valid|name/i);
		expect(calls).toHaveLength(0);
	});
});

describe('stale-edit guard', () => {
	it('sends back the ETag it read as If-Match when saving', async () => {
		answers.push({ status: 200, body: domain('g1', 'g.test'), etag: '"v1"' });
		answers.push({ status: 200, body: { ...domain('g1', 'g.test'), description: 'x' }, etag: '"v2"' });
		answers.push({ status: 200, body: { ...domain('g1', 'g.test'), description: 'y' }, etag: '"v3"' });
		await loadRegistryPage('x:Domain', { ids: ['g1'] });
		await writeRegistry('x:Domain', { update: { g1: { description: 'x' } } }, 'g1');
		await writeRegistry('x:Domain', { update: { g1: { description: 'y' } } }, 'g1');
		expect(calls[1].ifMatch).toBe('"v1"');
		// the answer to a save carries the new tag, so a second save in a row is not stale
		expect(calls[2].ifMatch).toBe('"v2"');
	});

	it('does not guard a record it never read', async () => {
		answers.push({ status: 200, body: domain('g2', 'h.test') });
		await writeRegistry('x:Domain', { update: { g2: { description: 'x' } } }, 'g2');
		expect(calls[0].ifMatch).toBeUndefined();
	});

	it('guards a delete too, and says why a stale one was refused', async () => {
		answers.push({ status: 200, body: domain('g3', 'i.test'), etag: '"a"' });
		answers.push({ status: 412, body: { type: 'stale', title: 'Changed', status: 412 } });
		await loadRegistryPage('x:Domain', { ids: ['g3'] });
		await expect(writeRegistry('x:Domain', { destroy: ['g3'] }, 'g3')).rejects.toThrow(
			/changed by someone else/
		);
		expect(calls[1].ifMatch).toBe('"a"');
	});
});
