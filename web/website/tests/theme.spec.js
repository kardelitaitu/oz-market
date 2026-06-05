import { test, expect } from '@playwright/test';

test.describe('Theme Switcher & UI Interactions', () => {
  test.beforeEach(async ({ page }) => {
    // Clear localStorage to ensure a clean slate
    await page.goto('/');
    await page.evaluate(() => localStorage.clear());
    await page.goto('/');
  });

  test('should load with default theme "midnight"', async ({ page }) => {
    const body = page.locator('body');
    await expect(body).toHaveAttribute('data-theme', 'midnight');
  });

  test('should change theme and persist across reloads', async ({ page }) => {
    const body = page.locator('body');
    const select = page.locator('.theme-select');

    // Change theme to emerald
    await select.selectOption('emerald');
    await expect(body).toHaveAttribute('data-theme', 'emerald');

    // Reload page and check if it persists
    await page.reload();
    await expect(body).toHaveAttribute('data-theme', 'emerald');

    // Cycle through all 5 themes
    const themes = ['midnight', 'emerald', 'crimson', 'solar', 'nordic'];
    for (const theme of themes) {
      await select.selectOption(theme);
      await expect(body).toHaveAttribute('data-theme', theme);
    }
  });

  test('should toggle simulator autoplay pause', async ({ page }) => {
    const pauseBtn = page.locator('button:has-text("Autoplay")');
    await expect(pauseBtn).toBeVisible();

    // Check initial state
    await expect(pauseBtn).toContainText('Pause Autoplay');

    // Click to pause
    await pauseBtn.click();
    await expect(pauseBtn).toContainText('Resume Autoplay');

    // Click to resume
    await pauseBtn.click();
    await expect(pauseBtn).toContainText('Pause Autoplay');
  });

  test('should navigate between tabs', async ({ page }) => {
    const guideTabBtn = page.locator('nav button:has-text("Device Guide")');
    await guideTabBtn.click();
    await expect(page.locator('h2:has-text("Multi-Device Setup Guide")')).toBeVisible();

    const docsTabBtn = page.locator('nav button:has-text("Documentation")');
    await docsTabBtn.click();
    await expect(page.locator('h2:has-text("Documentation Hub")')).toBeVisible();
  });
});
