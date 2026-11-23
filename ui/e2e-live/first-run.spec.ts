import { expect, test } from '@playwright/test';

// Run after the installer has claimed a disposable PostgreSQL instance.
const url = process.env.FIRST_RUN_URL;
const password = process.env.FIRST_RUN_PASSWORD;
test.skip(!url || !password, 'set FIRST_RUN_URL and FIRST_RUN_PASSWORD for an installer fixture');
test.use({ baseURL: url });

test('installed owner signs in normally without a setup credential or second claim', async ({ page }) => {
	const setup = await (await fetch(`${url}/api/v1/setup`)).json();
	expect(setup.pending).toBe(false);
	expect(setup.administrator).toBe('root@system.local');
	await page.goto('/login');
	await page.getByLabel('Account name').fill('root@system.local');
	await page.getByRole('button', { name: 'Continue' }).click();
	await page.getByLabel('Password').fill(password!);
	await page.getByRole('button', { name: 'Sign in' }).click();
	await page.waitForURL(/\/shippin\/admin/, { timeout: 30_000 });
	await expect(page.getByRole('heading', { name: 'Create your administrator' })).toHaveCount(0);
	await expect(
		page.getByRole('navigation', { name: 'Main navigation' }).getByRole('link', { name: 'Admin' })
	).toBeVisible();
	const again = await fetch(`${url}/api/v1/setup/admin`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ password: 'Unaccepted replacement fixture' })
	});
	expect(again.status).toBe(409);
});
