import { expect, test } from '@playwright/test';

test('first-run setup creates the fixed system identity without submitting a mail domain', async ({
	page
}) => {
	await page.addInitScript(() => {
		sessionStorage.setItem('shippin-basic', btoa('admin:synthetic-fixture'));
		sessionStorage.setItem('shippin-account', JSON.stringify({ username: 'admin', accountId: 'a' }));
	});
	let submitted: unknown;
	await page.route('**/jmap', (route) => route.fulfill({ json: { methodResponses: [] } }));
	await page.route('**/api/**', (route) => {
		const path = new URL(route.request().url()).pathname;
		if (path === '/api/v1/setup')
			return route.fulfill({ json: { pending: !submitted, administrator: 'root@system.local' } });
		if (path === '/api/v1/setup/admin') {
			submitted = route.request().postDataJSON();
			return route.fulfill({ json: { email: 'root@system.local' } });
		}
		return route.fulfill({ status: 404, json: {} });
	});
	await page.goto('/shippin/admin');
	await expect(page.getByRole('heading', { name: 'Create your administrator' })).toBeVisible();
	await expect(page.getByLabel('System administrator')).toHaveValue('root@system.local');
	await expect(page.getByLabel('System administrator')).not.toBeEditable();
	await page.getByLabel('Password', { exact: true }).fill('system administrator fixture');
	await page.getByLabel('Repeat password').fill('different fixture');
	await page.getByRole('button', { name: 'Create administrator' }).click();
	await expect(page.getByRole('alert')).toContainText('The two passwords are different.');
	expect(submitted).toBeUndefined();
	await page.getByLabel('Repeat password').fill('system administrator fixture');
	await page.getByRole('button', { name: 'Create administrator' }).click();
	await expect(page).toHaveURL(/\/login\?.*admin=1/);
	expect(submitted).toEqual({ password: 'system administrator fixture' });
	await expect(page.getByLabel('Account name')).toHaveValue('root@system.local');
});
