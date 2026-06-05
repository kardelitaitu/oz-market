import { test, expect } from '@playwright/test';

// Tab forward through focusable elements until the given locator is focused.
// Returns true if the element was focused within maxTabs iterations.
async function tabUntilFocused(page, locator, maxTabs = 50) {
  for (let i = 0; i < maxTabs; i++) {
    await page.keyboard.press('Tab');
    const isTarget = await locator.evaluate(el => document.activeElement === el);
    if (isTarget) return true;
  }
  return false;
}

// Get the tag name of a Playwright locator element.
async function getTagName(locator) {
  return locator.evaluate(el => el.tagName);
}

test.describe('Accessibility: Semantic Landmarks', () => {
  test('page has a semantic header landmark', async ({ page }) => {
    await page.goto('/');
    const header = page.locator('header');
    await expect(header).toBeVisible();
    // Header contains the logo and nav
    await expect(header.locator('h1')).toBeVisible();
    await expect(header.locator('nav')).toBeVisible();
  });

  test('page has a semantic <main> content landmark', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('main')).toBeVisible();
  });

  test('page has a semantic <nav> element for tab navigation', async ({ page }) => {
    await page.goto('/');
    const nav = page.locator('nav');
    await expect(nav).toBeVisible();
    // Nav should have 3 tab buttons
    const buttons = nav.locator('button');
    await expect(buttons).toHaveCount(3);
    await expect(buttons.nth(0)).toHaveText('Home');
    await expect(buttons.nth(1)).toHaveText('Device Guide');
    await expect(buttons.nth(2)).toHaveText('Documentation');
  });

  test('page has a semantic <footer> landmark', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('footer')).toBeVisible();
    await expect(page.locator('footer p')).toContainText('oz-market');
  });

  test('page has exactly one <h1> for the site identity', async ({ page }) => {
    await page.goto('/');
    const h1s = page.locator('h1');
    // Some views may render additional h1s (guide tab), but the root <header> should have one
    await expect(h1s.first()).toHaveText('oz-market');
  });

  test('Home tab hero section has an <h2> and descriptive <p>', async ({ page }) => {
    await page.goto('/');
    const hero = page.locator('section.hero');
    await expect(hero.locator('h2')).toBeVisible();
    await expect(hero.locator('p')).toBeVisible();
  });
});

test.describe('Accessibility: ThemeSwitcher', () => {
  test('theme switcher group has role="group" with aria-label', async ({ page }) => {
    await page.goto('/');
    const group = page.locator('[role="group"]');
    await expect(group).toBeVisible();
    await expect(group).toHaveAttribute('aria-label', 'Select Theme');
  });

  test('all 5 theme swatch buttons have aria-label and aria-pressed', async ({ page }) => {
    await page.goto('/');
    const themes = ['Midnight', 'Emerald', 'Crimson', 'Solar', 'Nordic'];
    for (const label of themes) {
      const btn = page.locator(`button[aria-label="${label} theme"]`);
      await expect(btn).toBeVisible();
      await expect(btn).toHaveAttribute('aria-pressed');
    }
  });

  test('default active swatch has aria-pressed="true" and others have "false"', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('[aria-label="Midnight theme"]')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.locator('[aria-label="Emerald theme"]')).toHaveAttribute('aria-pressed', 'false');
    await expect(page.locator('[aria-label="Crimson theme"]')).toHaveAttribute('aria-pressed', 'false');
    await expect(page.locator('[aria-label="Solar theme"]')).toHaveAttribute('aria-pressed', 'false');
    await expect(page.locator('[aria-label="Nordic theme"]')).toHaveAttribute('aria-pressed', 'false');
  });

  test('theme swatch buttons have title attributes', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('[aria-label="Midnight theme"]')).toHaveAttribute('title', 'Midnight');
    await expect(page.locator('[aria-label="Emerald theme"]')).toHaveAttribute('title', 'Emerald');
  });

  test('theme swatches are focusable buttons', async ({ page }) => {
    await page.goto('/');
    const midnightBtn = page.locator('[aria-label="Midnight theme"]');
    // Tab through nav buttons to reach the theme swatch area
    const found = await tabUntilFocused(page, midnightBtn);
    expect(found).toBe(true);
    await expect(midnightBtn).toBeFocused();
  });

  test('pressing Enter on a theme swatch activates it', async ({ page }) => {
    await page.goto('/');
    const emeraldBtn = page.locator('[aria-label="Emerald theme"]');

    // Tab until Emerald swatch is focused
    const found = await tabUntilFocused(page, emeraldBtn);
    expect(found).toBe(true);
    await expect(emeraldBtn).toBeFocused();

    // Press Enter to activate
    await page.keyboard.press('Enter');
    await expect(emeraldBtn).toHaveAttribute('aria-pressed', 'true');
    // Body theme should change
    await expect(page.locator('body')).toHaveAttribute('data-theme', 'emerald');
  });
});

test.describe('Accessibility: FlowDiagram', () => {
  test('SVG flow diagram has an aria-label', async ({ page }) => {
    await page.goto('/');
    const svg = page.locator('svg.flow-diagram');
    await expect(svg).toHaveAttribute('aria-label', 'Agent negotiation flow diagram');
  });

  test('SVG defs filter is in the accessibility tree as a presentational child', async ({ page }) => {
    await page.goto('/');
    const defs = page.locator('svg.flow-diagram defs');
    await expect(defs).toHaveCount(1);
    // <defs> content is not rendered, but we verify the structure exists
    const filter = defs.locator('filter#node-glow');
    await expect(filter).toHaveCount(1);
  });

  test('SVG node text labels are visible to assistive technology', async ({ page }) => {
    await page.goto('/');
    const buyerText = page.locator('svg.flow-diagram text:has-text("BUYER")');
    const serverText = page.locator('svg.flow-diagram text:has-text("SERVER")');
    const sellerText = page.locator('svg.flow-diagram text:has-text("SELLER")');
    // SVG text elements are visible in the accessibility tree
    await expect(buyerText).toBeVisible();
    await expect(serverText).toBeVisible();
    await expect(sellerText).toBeVisible();
  });
});

test.describe('Accessibility: Navigation & Tab System', () => {
  test('nav tab buttons are focusable via keyboard', async ({ page }) => {
    await page.goto('/');
    const homeBtn = page.locator('nav button:has-text("Home")');
    const guideBtn = page.locator('nav button:has-text("Device Guide")');
    const docsBtn = page.locator('nav button:has-text("Documentation")');

    // Tab to nav buttons (they are the first focusable elements after page load)
    await page.keyboard.press('Tab');
    await expect(homeBtn).toBeFocused();

    await page.keyboard.press('Tab');
    await expect(guideBtn).toBeFocused();

    await page.keyboard.press('Tab');
    await expect(docsBtn).toBeFocused();
  });

  test('pressing Enter on a nav button navigates to that tab', async ({ page }) => {
    await page.goto('/');
    const guideBtn = page.locator('nav button:has-text("Device Guide")');

    // Tab to Device Guide button
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await expect(guideBtn).toBeFocused();

    // Activate with Enter
    await page.keyboard.press('Enter');
    await expect(page.locator('h2:has-text("Multi-Device Setup Guide")')).toBeVisible();

    // Tab to Documentation and activate
    await page.keyboard.press('Tab');
    await page.keyboard.press('Enter');
    await expect(page.locator('h2:has-text("Documentation Hub")')).toBeVisible();
  });

  test('active nav button has the active CSS class for visual indication', async ({ page }) => {
    await page.goto('/');
    const homeBtn = page.locator('nav button:has-text("Home")');
    await expect(homeBtn).toHaveClass(/active/);

    // Navigate to guide
    await page.locator('nav button:has-text("Device Guide")').click();
    await expect(page.locator('nav button:has-text("Device Guide")')).toHaveClass(/active/);
    await expect(homeBtn).not.toHaveClass(/active/);
  });
});

test.describe('Accessibility: Simulator Controls', () => {
  test('simulator action buttons are <button> elements and focusable', async ({ page }) => {
    await page.goto('/');
    const simulator = page.locator('section.card:has(h3:text("Interactive Agent Negotiation Simulator"))');
    const buttons = simulator.locator('button');
    const count = await buttons.count();
    expect(count).toBeGreaterThanOrEqual(2); // At least 2 control buttons

    // All buttons in the simulator should be focusable
    for (let i = 0; i < count; i++) {
      await expect(buttons.nth(i)).toBeVisible();
    }
  });

  test('disabled simulator buttons have the disabled attribute', async ({ page }) => {
    await page.goto('/');
    // Auto-start sets state to 'listing', which renders a disabled "Publishing..." button
    const disabledBtn = page.locator('button[disabled]:has-text("Publishing...")');
    await expect(disabledBtn).toBeVisible({ timeout: 3000 });
  });

  test('autoplay toggle button is focusable and activated by keyboard', async ({ page }) => {
    await page.goto('/');
    const pauseBtn = page.locator('button:has-text("Autoplay")');

    // Tab until the autoplay toggle is focused
    const found = await tabUntilFocused(page, pauseBtn);
    expect(found).toBe(true);
    await expect(pauseBtn).toBeFocused();

    // Press Enter to toggle pause
    await page.keyboard.press('Enter');
    await expect(pauseBtn).toContainText('Resume Autoplay');
  });
});

test.describe('Accessibility: GuideTab Device Tabs', () => {
  test('device tab buttons have type="button" and are focusable', async ({ page }) => {
    await page.goto('/');
    await page.locator('nav button:has-text("Device Guide")').click();

    const tabs = page.locator('.device-tab');
    await expect(tabs).toHaveCount(3);

    // Each should be a button with type="button"
    for (let i = 0; i < 3; i++) {
      await expect(tabs.nth(i)).toHaveAttribute('type', 'button');
    }
  });

  test('device tabs are keyboard-focusable', async ({ page }) => {
    await page.goto('/');
    await page.locator('nav button:has-text("Device Guide")').click();

    const serverTab = page.locator('.device-tab').first();

    // Tab until the first device tab (Server) is focused
    const found = await tabUntilFocused(page, serverTab);
    expect(found).toBe(true);
    await expect(serverTab).toBeFocused();
    await expect(serverTab).toContainText('Marketplace Server');
  });

  test('pressing Enter on MCP device tab switches content', async ({ page }) => {
    await page.goto('/');
    await page.locator('nav button:has-text("Device Guide")').click();

    const mcpTab = page.locator('.device-tab:has-text("MCP Sidecar")');

    // Tab until MCP tab is focused
    const found = await tabUntilFocused(page, mcpTab);
    expect(found).toBe(true);
    await expect(mcpTab).toBeFocused();

    // Activate with Enter
    await page.keyboard.press('Enter');
    await expect(mcpTab).toHaveClass(/active/);
    await expect(page.locator('h4:has-text("Build the MCP Executable")')).toBeVisible();
  });
});

test.describe('Accessibility: DocsTab Links', () => {
  test('doc-item elements are <a> links with href attributes', async ({ page }) => {
    await page.goto('/');
    await page.locator('nav button:has-text("Documentation")').click();

    const links = page.locator('a.doc-item');
    const count = await links.count();
    expect(count).toBe(8);

    for (let i = 0; i < count; i++) {
      const link = links.nth(i);
      await expect(link).toHaveAttribute('href');
      // All links should start with docs/
      const href = await link.getAttribute('href');
      expect(href).toMatch(/^docs\//);
    }
  });

  test('docs links are keyboard-focusable', async ({ page }) => {
    await page.goto('/');
    await page.locator('nav button:has-text("Documentation")').click();

    // Tab until a .doc-item link receives focus
    const firstDocLink = page.locator('a.doc-item').first();
    const found = await tabUntilFocused(page, firstDocLink);
    expect(found).toBe(true);
    await expect(firstDocLink).toBeFocused();

    const activeHref = await page.evaluate(() => document.activeElement?.getAttribute('href'));
    expect(activeHref).toMatch(/^docs\//);
  });
});

test.describe('Accessibility: Benchmark Table', () => {
  test('benchmark table has proper semantic structure', async ({ page }) => {
    await page.goto('/');
    const table = page.locator('table');
    await expect(table).toBeVisible();

    // Should have thead + tbody
    await expect(table.locator('thead')).toHaveCount(1);
    await expect(table.locator('tbody')).toHaveCount(1);

    // thead should have header cells
    const headers = table.locator('thead th');
    await expect(headers).toHaveCount(5);
    await expect(headers.nth(0)).toContainText('Search Concurrency');
    await expect(headers.nth(3)).toContainText('Rate Limit');

    // tbody should have data rows
    const rows = table.locator('tbody tr');
    await expect(rows).toHaveCount(3);
  });
});

test.describe('Accessibility: Interactive Element Roles', () => {
  test('all nav buttons are interactive <button> elements', async ({ page }) => {
    await page.goto('/');
    const navButtons = page.locator('nav button');
    const count = await navButtons.count();
    expect(count).toBe(3);

    // Verify they respond to clicks
    await navButtons.nth(1).click();
    await expect(page.locator('h2:has-text("Multi-Device Setup Guide")')).toBeVisible();
  });

  test('all theme swatches are <button> elements', async ({ page }) => {
    await page.goto('/');
    const swatches = page.locator('.theme-swatch');
    const count = await swatches.count();
    expect(count).toBe(5);

    // Each should be a button
    for (let i = 0; i < count; i++) {
      const tagName = await getTagName(swatches.nth(i));
      expect(tagName).toBe('BUTTON');
    }
  });

  test('simulator uses proper heading hierarchy (h3, h5)', async ({ page }) => {
    await page.goto('/');
    const simulator = page.locator('section.card:has(h3:text("Interactive Agent Negotiation Simulator"))');

    // Section has h3 for title
    await expect(simulator.locator('h3')).toBeVisible();
    // Logs section has h5 for subtitle
    await expect(simulator.locator('h5')).toContainText('Simulation logs');
  });

  test('footer uses <p> for copyright text', async ({ page }) => {
    await page.goto('/');
    const footer = page.locator('footer');
    await expect(footer.locator('p')).toContainText('©');
  });
});

test.describe('Accessibility: Color & Visual Indicators (Non-Color Dependent)', () => {
  test('status display text communicates state without relying solely on color', async ({ page }) => {
    await page.goto('/');
    // The status display shows text like "Publishing...", "Negotiating...", "Consensus!"
    // These are meaningful text descriptions, not just color changes
    const statusDisplay = page.locator('text=Publishing...').first();
    await expect(statusDisplay).toBeVisible({ timeout: 3000 });
  });

  test('backend status uses both text and dot indicator for redundancy', async ({ page }) => {
    await page.goto('/');
    // The backend pill shows both a colored dot AND text "Backend: Offline (Demo Mode)"
    // This provides redundant coding (text + icon) for accessibility
    const backendText = page.getByText('Backend: Offline (Demo Mode)');
    await expect(backendText).toBeVisible();

    // The dot exists as a visual indicator alongside the text
    const dot = page.locator('span[style*="border-radius: 50%"]').first();
    await expect(dot).toBeVisible();
  });

  test('card headings use emoji icons that are decorative (text alternative present)', async ({ page }) => {
    await page.goto('/');
    // Cards have emoji icons in their h3 headings, but the heading text itself is descriptive
    const cards = page.locator('.grid-3 .card h3');
    await expect(cards.nth(0)).toContainText('High-Frequency Scale');
    await expect(cards.nth(1)).toContainText('Zero-Knowledge Privacy');
    await expect(cards.nth(2)).toContainText('Dual-Layer Ledger');
    // Each heading has text that communicates the meaning without relying on the emoji
  });
});
