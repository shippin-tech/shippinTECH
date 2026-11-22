// The admin screens' registry reads and writes, through /api/v1/registry (REST over native JMAP)
// and the generated contracts. `x:Name` objects go here; the standard JMAP types (Mailbox, Calendar,
// Principal, ...) stay on JMAP, because the facade does not carry them (api/DESIGN.md).
//
// The contracts are loaded on first use, so the generated runtime file is its own chunk and is not
// part of the entry bundle.
import type { Problem } from '../contract/v1';
import { authorization, clearSession } from './session';
import type { RegistryChange } from './writes-call';

const PREFIX = 'x:';

/** The facade's name for an admin object name, or null when it is not a registry object. */
export function facadeName(object: string): string | null {
	return object.startsWith(PREFIX) ? object.slice(PREFIX.length) : null;
}

export async function client() {
	const auth = authorization();
	if (!auth) throw new Error('sign-in required');
	const { registryClient, isRegistryName } = await import('../contract/registry');
	return { c: registryClient({ authorization: () => authorization() ?? undefined }), isRegistryName };
}

/** Every private admin object uses our API, including unknown names that it must refuse. */
export async function servedByFacade(object: string): Promise<boolean> {
	return facadeName(object) !== null;
}

/** A refusal as the Error the screens already show. A 401 ends the session, as the JMAP path does. */
export function refusal(problem: Problem): Error {
	if (problem.status === 401) {
		clearSession();
		return new Error('sign-in required');
	}
	if (problem.type === 'primaryKeyViolation') return new Error('That name is already taken.');
	if (problem.type === 'objectIsLinked') {
		return new Error(
			'This record is still referenced by other objects. Remove those links before deleting it.'
		);
	}
	if (problem.type === 'stale') {
		return new Error(
			'This record was changed by someone else after you opened it. Reload it and make your change again.'
		);
	}
	const fields = problem.fields ? Object.keys(problem.fields) : [];
	const fieldText = fields.length ? ` (${fields.map((f) => `${f}: ${problem.fields?.[f]}`).join('; ')})` : '';
	// A record that broke its contract is the server's fault, not the person's: say so first.
	if (problem.type === 'malformed-response') {
		return new Error(`${problem.title}: ${problem.detail ?? 'a record did not match its schema'}`);
	}
	return new Error((problem.detail || problem.title || problem.type) + fieldText);
}

export type FacadeList = {
	filter?: Record<string, unknown>;
	limit: number;
	position: number;
	sortBy: string;
	sortAscending: boolean;
	ids?: string[];
};

export type FacadePage = { items: Record<string, unknown>[]; total: number | null };

export async function facadeList(object: string, options: FacadeList): Promise<FacadePage> {
	const name = facadeName(object);
	if (name === null) throw new Error(`${object} is not a registry object`);
	const { c } = await client();

	if (options.ids?.length) {
		// A singleton is one record with a fixed id; any other ids are fetched one by one.
		if (options.ids.length === 1 && options.ids[0] === 'singleton') {
			const one = await c.any.get(name);
			if (!one.ok) throw refusal(one.problem);
			return { items: [one.value], total: 1 };
		}
		const found = await Promise.all(options.ids.map((id) => c.any.get(name, id)));
		const items: Record<string, unknown>[] = [];
		for (const result of found) {
			if (result.ok) items.push(result.value);
			else if (result.problem.status !== 404) throw refusal(result.problem);
		}
		return { items, total: items.length };
	}

	const result = await c.any.list(name, {
		limit: options.limit,
		position: options.position,
		filter: options.filter,
		sort: options.sortBy ? `${options.sortAscending ? '' : '-'}${options.sortBy}` : undefined
	});
	if (!result.ok) throw refusal(result.problem);
	return { items: result.value.items, total: result.value.total ?? null };
}

/** Applies one change and returns the id of the record it made or changed. */
export async function facadeWrite(object: string, change: RegistryChange, id: string): Promise<string> {
	const name = facadeName(object);
	if (name === null) throw new Error(`${object} is not a registry object`);
	const { c } = await client();

	if ('create' in change) {
		const input = change.create[id] ?? Object.values(change.create)[0] ?? {};
		const made = await c.any.create(name, input);
		if (!made.ok) throw refusal(made.problem);
		return typeof made.value.id === 'string' ? made.value.id : id;
	}
	if ('update' in change) {
		const target = Object.keys(change.update).includes(id) ? id : Object.keys(change.update)[0];
		const patch = change.update[target] ?? {};
		const changed = await c.any.update(name, target, patch);
		if (!changed.ok) throw refusal(changed.problem);
		return target;
	}
	for (const target of change.destroy) {
		const removed = await c.any.remove(name, target);
		if (!removed.ok) throw refusal(removed.problem);
	}
	return id;
}
