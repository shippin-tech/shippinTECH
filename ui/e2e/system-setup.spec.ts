import { expect, test } from '@playwright/test';

test('unclaimed server directs its installer owner to finish locally', async ({ page }) => {
	let claims = 0;
	await page.route('**/api/**', (route) => {
		const path = new URL(route.request().url()).pathname;
		if (path === '/api/v1/setup')
			return route.fulfill({ json: { pending: true, administrator: 'root@system.local' } });
		if (path === '/api/v1/setup/admin') claims++;
		return route.fulfill({ status: 404, json: {} });
	});
	await page.goto('/login');
	await expect(page.getByRole('status')).toContainText('bash claim.sh');
	await expect(page.getByText('Paste your setup key here.', { exact: false })).toHaveCount(0);
	await expect(page.getByRole('heading', { name: 'Create your administrator' })).toHaveCount(0);
	await expect(page.getByLabel('Account name')).toBeVisible();
	expect(claims).toBe(0);
});
