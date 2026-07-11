import { test, expect } from '@playwright/test';

test.describe('Shop MVP — smoke tests', () => {

  test('Pagina principală se încarcă', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('h1')).toContainText('Shop MVP');
  });

  test('Pagina de produse se încarcă', async ({ page }) => {
    await page.goto('/products');
    await expect(page.locator('h1')).toContainText('Produse');
  });

  test('Butonul + Coș folosește HTMX', async ({ page }) => {
    await page.goto('/products');
    const btn = page.locator('button:has-text("+ Coș")').first();
    await expect(btn).toHaveAttribute('hx-post', /\/cart\/add/);
  });

  test('Navigare HTMX — click Coș', async ({ page }) => {
    await page.goto('/');
    await page.click('a:has-text("Coș")');
    await expect(page.locator('h1')).toContainText('Coșul meu');
  });

  test('Login formularul trimite prin HTMX', async ({ page }) => {
    await page.goto('/login');
    const form = page.locator('form[hx-post]');
    await expect(form).toHaveCount(1);
    // Verifică că inputurile există
    await expect(page.locator('input[name="email"]')).toBeVisible();
    await expect(page.locator('input[name="password"]')).toBeVisible();
    await expect(page.locator('button[type="submit"]')).toBeVisible();
  });

  test('Admin — redirect la login fără token', async ({ page }) => {
    await page.goto('/admin');
    // Fără token, ar trebui să vadă login sau eroare
    await expect(page).toHaveURL(/login/);
  });

});
