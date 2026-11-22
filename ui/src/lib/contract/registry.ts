// Client for /api/v1/registry (a thin REST facade over native JMAP; api/DESIGN.md).
//
// What it promises:
//  - What goes out is checked against the generated create/update contract first. A value the
//    contract refuses is never sent; the refusal has the same Problem shape a server refusal has.
//  - What comes back is checked against the generated read contract. Undeclared keys are dropped
//    at every depth, so the app only sees what the schema names.
//  - The HTTP status decides success or failure; the body is then checked for that outcome.
//  - Nothing throws. A network failure is a Problem too.
import { type Type, type } from 'arktype';
import { type RegistryName, registry } from './registry.generated';
import { fieldMessages, localProblem, type Parsed, type Problem, parseResponse } from './v1';

type Registry = typeof registry;
type Read<N extends RegistryName> = ReturnType<Registry[N]['read']>['infer'];
type Create<N extends RegistryName> = ReturnType<Registry[N]['create']>['infer'];
type Update<N extends RegistryName> = ReturnType<Registry[N]['update']>['infer'];

export type Page<T> = { items: T[]; position: number; total?: number };
/** `sort` is one property name; a leading `-` sorts descending. */
export type ListOptions = {
	limit?: number;
	position?: number;
	filter?: Record<string, unknown>;
	sort?: string;
};

export type ClientOptions = {
	/** The Authorization header value, or nothing when signed out. */
	authorization: () => string | undefined;
	fetch?: typeof fetch;
	base?: string;
};

const asProblem = (errors: type.errors): Problem =>
	localProblem(
		'invalid-input',
		'That was not sent: it is not valid',
		422,
		errors.summary,
		fieldMessages(errors)
	);

/** The page envelope around a read contract. Built per call: it is two keys and one array. */
function pageOf<T>(item: Type<T>) {
	return type({
		items: item.array(),
		position: 'number.integer >= 0',
		'total?': 'number.integer >= 0'
	}).onDeepUndeclaredKey('delete');
}

/**
 * The one place a generic object name is related to its generated contracts. TypeScript cannot
 * follow `registry[name]` through 117 differently-typed entries, so the entry is read as the
 * shape every entry has. What is checked at run time is exactly what the generator emitted.
 */
function contractsOf<N extends RegistryName>(name: N) {
	const entry = registry[name] as unknown as {
		read: () => Type<Read<N>>;
		create: () => Type<Create<N>>;
		update: () => Type<Update<N>>;
	};
	return entry;
}

type Row = Record<string, unknown>;

// Preserve the stored authenticator when changing only the password or OTP code.
const AccountPasswordPatch = type({
	'currentSecret?': 'string',
	'secret?': 'string',
	'otpAuth/otpCode?': 'string',
	'otpAuth/otpUrl?': 'string | null'
}).onDeepUndeclaredKey('reject');
export type AccountPasswordPatch = typeof AccountPasswordPatch.infer;

const CreatedAccount = type({ id: 'string > 0' }).onDeepUndeclaredKey('delete');

/** Validate person edits using the generated schema, retaining credential field paths. */
function personPatch(input: Row): Parsed<Row> {
	const schema = registry.Account.update()
		.extract({ '@type': "'User'" })
		.omit('@type')
		.onDeepUndeclaredKey('reject');
	const ordinary = Object.fromEntries(Object.entries(input).filter(([key]) => !key.includes('/')));
	const checked = schema(ordinary);
	if (checked instanceof type.errors) return { ok: false, problem: asProblem(checked) };
	const patch: Row = { ...checked };
	const credentials = new Set<string>();
	for (const [path, value] of Object.entries(input)) {
		if (!path.includes('/')) continue;
		const match = /^credentials\/([A-Za-z0-9_-]+)(\/secret)?$/.exec(path);
		if (!match || 'credentials' in ordinary || credentials.has(match[1])) {
			return { ok: false, problem: localProblem('invalid-input', 'Invalid credential patch', 422) };
		}
		credentials.add(match[1]);
		const entry = match[2] ? { '@type': 'Password', secret: value } : value;
		const credential = schema({ credentials: { [match[1]]: entry } });
		if (credential instanceof type.errors) return { ok: false, problem: asProblem(credential) };
		// Validation does not expand the patch: the engine must see only the named field.
		patch[path] = value;
	}
	return { ok: true, value: patch };
}

/** True for a name the generated registry has. */
export function isRegistryName(name: string): name is RegistryName {
	return Object.hasOwn(registry, name);
}

/** What the screens use: the same entry as `contractsOf`, read as plain records. */
function loose(name: RegistryName) {
	return registry[name] as unknown as {
		read: () => Type<Row>;
		create: () => Type<Row>;
		update: () => Type<Row>;
	};
}

const unknownObject = (name: string): Parsed<never> => ({
	ok: false,
	problem: localProblem('unknown-object', 'There is no such object', 404, name)
});

// The ETag of each record this page has read or saved, by URL. A save or a delete sends it back as
// If-Match, so the server refuses (412) when the record changed after it was read. A record that was
// never read in this page is saved without the guard.
const tags = new Map<string, string>();

export function registryClient(options: ClientOptions) {
	const doFetch = options.fetch ?? fetch;
	const base = options.base ?? '/api/v1/registry';

	async function send<T>(
		method: string,
		path: string,
		schema: Type<T> | null,
		payload?: unknown
	): Promise<Parsed<Type<T>['infer'] | undefined>> {
		const headers: Record<string, string> = { Accept: 'application/json' };
		const authorization = options.authorization();
		if (authorization) headers.Authorization = authorization;
		const tagKey = `${authorization ?? ''}\n${base}${path}`;
		if (method === 'PATCH' || method === 'DELETE') {
			const tag = tags.get(tagKey);
			if (tag) headers['If-Match'] = tag;
		}
		let body: string | undefined;
		if (payload !== undefined) {
			headers['Content-Type'] = 'application/json';
			body = JSON.stringify(payload);
		}
		let response: Response;
		try {
			response = await doFetch(`${base}${path}`, {
				method,
				headers,
				body,
				cache: 'no-store',
				credentials: 'same-origin'
			});
		} catch {
			return { ok: false, problem: localProblem('network-error', 'The server could not be reached', 503) };
		}
		const tag = response.headers.get('etag');
		if (response.ok && tag && (method === 'GET' || method === 'PATCH')) tags.set(tagKey, tag);
		// A 412 keeps the old tag on purpose: saving again without reading is refused again.
		else if ((method === 'DELETE' && response.ok) || response.status === 404) tags.delete(tagKey);
		const text = response.status === 204 ? '' : await response.text().catch(() => '');
		let parsed: unknown = null;
		if (text) {
			try {
				parsed = JSON.parse(text);
			} catch {
				parsed = null;
			}
		}
		return parseResponse(schema, response.status, parsed);
	}

	const where = (name: string, id?: string) =>
		`/${encodeURIComponent(name)}${id === undefined ? '' : `/${encodeURIComponent(id)}`}`;

	return {
		/** Creates acknowledge the id; a reread would expose a different response contract. */
		async createAccount(input: Row): Promise<Parsed<typeof CreatedAccount.infer>> {
			const checked = registry.Account.create()(input);
			if (checked instanceof type.errors) return { ok: false, problem: asProblem(checked) };
			return send('POST', where('Account'), CreatedAccount, checked) as Promise<
				Parsed<typeof CreatedAccount.infer>
			>;
		},

		async updatePerson(id: string, input: Row): Promise<Parsed<Read<'Account'>>> {
			const checked = personPatch(input);
			if (!checked.ok) return checked;
			return send('PATCH', where('Account', id), registry.Account.read(), checked.value) as Promise<
				Parsed<Read<'Account'>>
			>;
		},

		/** An action returns output fields, not a persisted record with id and @type. */
		async executeAction(input: Row): Promise<Parsed<Row>> {
			const checked = registry.Action.create()(input);
			if (checked instanceof type.errors) return { ok: false, problem: asProblem(checked) };
			const output = registry.Action.read()
				.extract({ '@type': type.unit(checked['@type']) })
				.omit('id', '@type');
			return send('POST', where('Action'), output, checked) as Promise<Parsed<Row>>;
		},

		async updatePassword(patch: AccountPasswordPatch): Promise<Parsed<Read<'AccountPassword'>>> {
			const checked = AccountPasswordPatch(patch);
			if (checked instanceof type.errors) return { ok: false, problem: asProblem(checked) };
			return send('PATCH', where('AccountPassword'), registry.AccountPassword.read(), checked) as Promise<
				Parsed<Read<'AccountPassword'>>
			>;
		},

		/** A collection's records, one page. */
		async list<N extends RegistryName>(name: N, opts: ListOptions = {}): Promise<Parsed<Page<Read<N>>>> {
			const read = contractsOf(name).read();
			const query = new URLSearchParams();
			if (opts.limit !== undefined) query.set('limit', String(opts.limit));
			if (opts.position !== undefined) query.set('position', String(opts.position));
			if (opts.filter) query.set('filter', JSON.stringify(opts.filter));
			if (opts.sort) query.set('sort', opts.sort);
			const suffix = query.size ? `?${query}` : '';
			return send('GET', `${where(name)}${suffix}`, pageOf(read)) as Promise<Parsed<Page<Read<N>>>>;
		},

		/** One record; for a singleton object, omit the id. */
		async get<N extends RegistryName>(name: N, id?: string): Promise<Parsed<Read<N>>> {
			return send('GET', where(name, id), contractsOf(name).read()) as Promise<Parsed<Read<N>>>;
		},

		async create<N extends RegistryName>(name: N, input: Create<N>): Promise<Parsed<Read<N>>> {
			const contracts = contractsOf(name);
			const checked = contracts.create()(input);
			if (checked instanceof type.errors) return { ok: false, problem: asProblem(checked) };
			return send('POST', where(name), contracts.read(), checked) as Promise<Parsed<Read<N>>>;
		},

		/** Changes the named properties. For a singleton object, omit the id. */
		async update<N extends RegistryName>(
			name: N,
			id: string | undefined,
			patch: Update<N>
		): Promise<Parsed<Read<N>>> {
			const contracts = contractsOf(name);
			const checked = contracts.update()(patch);
			if (checked instanceof type.errors) return { ok: false, problem: asProblem(checked) };
			return send('PATCH', where(name, id), contracts.read(), checked) as Promise<Parsed<Read<N>>>;
		},

		async remove(name: RegistryName, id: string): Promise<Parsed<undefined>> {
			return send('DELETE', where(name, id), null) as Promise<Parsed<undefined>>;
		},

		/**
		 * The same calls for a name known only at run time (a screen that edits whichever object the
		 * URL names). Same contracts, same checks; a name the registry does not have is refused here,
		 * before any request. Rows are plain records: the screen shows what the schema names.
		 */
		any: {
			async list(name: string, opts: ListOptions = {}): Promise<Parsed<Page<Row>>> {
				if (!isRegistryName(name)) return unknownObject(name);
				const read = loose(name).read();
				const query = new URLSearchParams();
				if (opts.limit !== undefined) query.set('limit', String(opts.limit));
				if (opts.position !== undefined) query.set('position', String(opts.position));
				if (opts.filter) query.set('filter', JSON.stringify(opts.filter));
				if (opts.sort) query.set('sort', opts.sort);
				const suffix = query.size ? `?${query}` : '';
				return send('GET', `${where(name)}${suffix}`, pageOf(read)) as Promise<Parsed<Page<Row>>>;
			},
			async get(name: string, id?: string): Promise<Parsed<Row>> {
				if (!isRegistryName(name)) return unknownObject(name);
				return send('GET', where(name, id), loose(name).read()) as Promise<Parsed<Row>>;
			},
			async create(name: string, input: Row): Promise<Parsed<Row>> {
				if (!isRegistryName(name)) return unknownObject(name);
				const contracts = loose(name);
				const checked = contracts.create()(input);
				if (checked instanceof type.errors) return { ok: false, problem: asProblem(checked) };
				return send('POST', where(name), contracts.read(), checked) as Promise<Parsed<Row>>;
			},
			async update(name: string, id: string | undefined, patch: Row): Promise<Parsed<Row>> {
				if (!isRegistryName(name)) return unknownObject(name);
				const contracts = loose(name);
				const checked = contracts.update()(patch);
				if (checked instanceof type.errors) return { ok: false, problem: asProblem(checked) };
				return send('PATCH', where(name, id), contracts.read(), checked) as Promise<Parsed<Row>>;
			},
			async remove(name: string, id: string): Promise<Parsed<undefined>> {
				if (!isRegistryName(name)) return unknownObject(name);
				return send('DELETE', where(name, id), null) as Promise<Parsed<undefined>>;
			}
		}
	};
}
