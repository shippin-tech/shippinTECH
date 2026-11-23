/** True until the installation owner is claimed. */
export async function setupPending(): Promise<boolean> {
	const res = await fetch('/api/v1/setup', { cache: 'no-store', headers: { Accept: 'application/json' } });
	if (!res.ok) return false;
	const body = (await res.json().catch(() => ({}))) as { pending?: unknown };
	return body.pending === true;
}
