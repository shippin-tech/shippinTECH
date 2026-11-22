import { client, refusal } from '../../adapter/registry-facade';
import {
	newPersonProperties,
	type Person,
	type PersonDraft,
	personPatch,
	sortPeople,
	type Team,
	type TeamDomain,
	type TeamDraft,
	teamSet,
	toPerson,
	toTeam
} from './team-call';

export * from './team-call';

async function allRecords(name: 'Account' | 'Domain'): Promise<Record<string, unknown>[]> {
	const { c } = await client();
	const rows: Record<string, unknown>[] = [];
	const limit = 200;
	for (;;) {
		const result = await c.any.list(name, { limit, position: rows.length });
		if (!result.ok) throw refusal(result.problem);
		const page = result.value;
		rows.push(...page.items);
		if (
			page.items.length === 0 ||
			(page.total !== undefined ? rows.length >= page.total : page.items.length < limit)
		)
			return rows;
	}
}

export async function loadDirectory(): Promise<{ people: Person[]; teams: Team[]; domains: TeamDomain[] }> {
	const [accounts, domains] = await Promise.all([allRecords('Account'), allRecords('Domain')]);
	return {
		people: sortPeople(accounts.flatMap((raw) => toPerson(raw) ?? [])),
		teams: accounts.flatMap((raw) => toTeam(raw) ?? []).sort((a, b) => a.name.localeCompare(b.name)),
		domains: domains
			.map((row) => ({ id: String(row.id), name: String(row.name) }))
			.sort((a, b) => a.name.localeCompare(b.name))
	};
}

export async function createPerson(draft: PersonDraft): Promise<void> {
	const { c } = await client();
	const result = await c.createAccount(newPersonProperties(draft));
	if (!result.ok) throw refusal(result.problem);
}

export async function updatePerson(person: Person, draft: PersonDraft): Promise<boolean> {
	const patch = personPatch(draft, person);
	if (Object.keys(patch).length === 0) return false;
	const { c } = await client();
	const result = await c.updatePerson(person.id, patch);
	if (!result.ok) throw refusal(result.problem);
	return true;
}

export async function removeAccount(id: string): Promise<void> {
	const { c } = await client();
	const result = await c.remove('Account', id);
	if (!result.ok) throw refusal(result.problem);
}

export async function createTeam(draft: TeamDraft): Promise<void> {
	const { c } = await client();
	const result = await c.createAccount({
		'@type': 'Group',
		name: draft.name.trim().toLowerCase(),
		domainId: draft.domainId,
		...(draft.description.trim() ? { description: draft.description.trim() } : {})
	});
	if (!result.ok) throw refusal(result.problem);
}

/** Adds or removes one person from one team, keeping every other membership. */
export async function setMembership(person: Person, teamId: string, member: boolean): Promise<void> {
	const teamIds = member
		? [...new Set([...person.teamIds, teamId])]
		: person.teamIds.filter((id) => id !== teamId);
	const { c } = await client();
	const result = await c.updatePerson(person.id, { memberGroupIds: teamSet(teamIds) });
	if (!result.ok) throw refusal(result.problem);
}
