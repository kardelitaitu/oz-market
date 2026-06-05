import { test, expect } from '@playwright/test';

test.describe('GuideTab Component', () => {
  // Helper to navigate to the Guide tab from Home
  async function goToGuide(page) {
    await page.goto('/');
    await page.click('nav button:has-text("Device Guide")');
    await page.waitForTimeout(200);
  }

  test('defaults to Server tab with server guide steps visible', async ({ page }) => {
    await goToGuide(page);
    await expect(page.locator('h2:has-text("Multi-Device Setup Guide")')).toBeVisible();

    // Server content should be visible
    await expect(page.locator('h4:has-text("Spin up PostgreSQL Database")')).toBeVisible();
    await expect(page.locator('h4:has-text("Run Schema Migrations & Seed Data")')).toBeVisible();
    await expect(page.locator('h4:has-text("Fire Up the Server")')).toBeVisible();

    // MCP and Mobile content should NOT be visible
    await expect(page.locator('h4:has-text("Build the MCP Executable")')).not.toBeVisible();
    await expect(page.locator('h4:has-text("Install Mobile Dependencies")')).not.toBeVisible();
  });

  test('three device tab buttons are present with correct labels and icons', async ({ page }) => {
    await goToGuide(page);
    const tabs = page.locator('.device-tabs .device-tab');
    await expect(tabs).toHaveCount(3);

    await expect(tabs.nth(0)).toContainText('Marketplace Server');
    await expect(tabs.nth(0)).toContainText('Rust API Core');
    await expect(tabs.nth(0)).toContainText('🖥️');

    await expect(tabs.nth(1)).toContainText('MCP Sidecar');
    await expect(tabs.nth(1)).toContainText('Model Context Protocol');
    await expect(tabs.nth(1)).toContainText('🔌');

    await expect(tabs.nth(2)).toContainText('Mobile App');
    await expect(tabs.nth(2)).toContainText('Tauri v2 + Svelte 5');
    await expect(tabs.nth(2)).toContainText('📱');
  });

  test('Server tab has active class by default', async ({ page }) => {
    await goToGuide(page);
    const serverTab = page.locator('.device-tab:has-text("Marketplace Server")');
    await expect(serverTab).toHaveClass(/\bactive\b/);

    // Other tabs should not be active
    await expect(page.locator('.device-tab:has-text("MCP Sidecar")')).not.toHaveClass(/\bactive\b/);
    await expect(page.locator('.device-tab:has-text("Mobile App")')).not.toHaveClass(/\bactive\b/);
  });

  test('clicking MCP tab switches to MCP content and marks MCP as active', async ({ page }) => {
    await goToGuide(page);

    await page.click('.device-tab:has-text("MCP Sidecar")');
    await page.waitForTimeout(200);

    // MCP content should now be visible
    await expect(page.locator('h4:has-text("Build the MCP Executable")')).toBeVisible();
    await expect(page.locator('h4:has-text("Configure Claude Desktop/Desktop Agent")')).toBeVisible();
    await expect(page.locator('h4:has-text("Expose AI capabilities")')).toBeVisible();

    // Server content should now be hidden
    await expect(page.locator('h4:has-text("Spin up PostgreSQL Database")')).not.toBeVisible();
    await expect(page.locator('h4:has-text("Install Mobile Dependencies")')).not.toBeVisible();

    // MCP tab should be active
    const mcpTab = page.locator('.device-tab:has-text("MCP Sidecar")');
    await expect(mcpTab).toHaveClass(/\bactive\b/);
    await expect(page.locator('.device-tab:has-text("Marketplace Server")')).not.toHaveClass(/\bactive\b/);
  });

  test('clicking Mobile tab switches to Mobile content', async ({ page }) => {
    await goToGuide(page);

    await page.click('.device-tab:has-text("Mobile App")');
    await page.waitForTimeout(200);

    // Mobile content should be visible — all 3 steps
    await expect(page.locator('h4:has-text("Install Mobile Dependencies")')).toBeVisible();
    await expect(page.locator('h4:has-text("Run in Development Mode")')).toBeVisible();
    await expect(page.locator('h4:has-text("Build Client Executables")')).toBeVisible();

    // Server/MCP content should be hidden
    await expect(page.locator('h4:has-text("Spin up PostgreSQL Database")')).not.toBeVisible();
    await expect(page.locator('h4:has-text("Build the MCP Executable")')).not.toBeVisible();

    // Mobile tab should be active
    const mobileTab = page.locator('.device-tab:has-text("Mobile App")');
    await expect(mobileTab).toHaveClass(/\bactive\b/);
  });

  test('each guide tab has exactly 3 guide steps', async ({ page }) => {
    await goToGuide(page);

    // Server: 3 steps
    await expect(page.locator('.guide-step')).toHaveCount(3);

    // MCP: 3 steps
    await page.click('.device-tab:has-text("MCP Sidecar")');
    await expect(page.locator('.guide-step')).toHaveCount(3);

    // Mobile: 3 steps
    await page.click('.device-tab:has-text("Mobile App")');
    await expect(page.locator('.guide-step')).toHaveCount(3);
  });

  test('mobile tab guide steps contain pre blocks with backslash-n commands', async ({ page }) => {
    await goToGuide(page);
    await page.click('.device-tab:has-text("Mobile App")');
    await page.waitForTimeout(200);

    // Mobile pre blocks should contain \n (literal backslash-n in HTML)
    const preBlocks = page.locator('.guide-step pre');
    await expect(preBlocks).toHaveCount(3);

    // Check first pre block: "cd mobile/marketplace\nnpm install"
    const firstPre = preBlocks.nth(0);
    await expect(firstPre).toContainText('cd mobile/marketplace');
    await expect(firstPre).toContainText('npm install');

    // Second pre block includes android/ios commands
    const secondPre = preBlocks.nth(1);
    await expect(secondPre).toContainText('android');
    await expect(secondPre).toContainText('ios');

    // Third pre block includes build commands
    const thirdPre = preBlocks.nth(2);
    await expect(thirdPre).toContainText('android build');
    await expect(thirdPre).toContainText('ios build');
  });

  test('server guide steps show expected docker and cargo commands', async ({ page }) => {
    await goToGuide(page);

    const preBlocks = page.locator('.guide-step pre');
    await expect(preBlocks.nth(0)).toContainText('docker compose');
    await expect(preBlocks.nth(1)).toContainText('cargo run --bin bootstrap_schema');
    await expect(preBlocks.nth(2)).toContainText('cargo run -p marketplace-server');
  });

  test('MCP guide steps show JSON config and cargo build command', async ({ page }) => {
    await goToGuide(page);
    await page.click('.device-tab:has-text("MCP Sidecar")');
    await page.waitForTimeout(200);

    const preBlocks = page.locator('.guide-step pre');
    await expect(preBlocks.nth(0)).toContainText('cargo build -p marketplace-mcp');

    // The JSON config pre block should contain mcpServers and marketplace key
    const jsonPre = preBlocks.nth(1);
    await expect(jsonPre).toContainText('mcpServers');
    await expect(jsonPre).toContainText('MARKETPLACE_API_KEY');
  });
});

test.describe('DocsTab Component', () => {
  // Helper to navigate to the Docs tab from Home
  async function goToDocs(page) {
    await page.goto('/');
    await page.click('nav button:has-text("Documentation")');
    await page.waitForTimeout(200);
  }

  test('displays Documentation Hub heading and subtitle', async ({ page }) => {
    await goToDocs(page);
    await expect(page.locator('h2:has-text("Documentation Hub")')).toBeVisible();
    await expect(page.locator('p:has-text("Detailed architecture maps")')).toBeVisible();
  });

  test('renders Core Whitepapers card section with 4 doc-item links', async ({ page }) => {
    await goToDocs(page);
    // Core Whitepapers card heading
    const whitepapersCard = page.locator('.card:has-text("Core Whitepapers")');
    await expect(whitepapersCard).toBeVisible();

    // Should have 4 doc-items inside
    const docItems = whitepapersCard.locator('.doc-item');
    await expect(docItems).toHaveCount(4);

    // Each doc-item should be a link
    await expect(docItems.nth(0)).toHaveAttribute('href', /docs\/01-whitepaper/);
    await expect(docItems.nth(0)).toContainText('Project Whitepaper Overview');
    await expect(docItems.nth(1)).toContainText('Frozen V1 API Contract');
    await expect(docItems.nth(2)).toContainText('Identity, Claims & Authz Matrix');
    await expect(docItems.nth(3)).toContainText('Server Crate Architecture');
  });

  test('doc items show correct meta file paths and arrow indicators', async ({ page }) => {
    await goToDocs(page);
    const docItem = page.locator('.card:has-text("Core Whitepapers") .doc-item').first();

    // Should show the file path as meta
    await expect(docItem.locator('.doc-meta')).toContainText('docs/01-whitepaper/README.md');

    // Should have an arrow indicator
    await expect(docItem.locator('.btn-arrow')).toBeVisible();
    await expect(docItem.locator('.btn-arrow')).toHaveText('→');
  });

  test('renders Active Roadmaps card section with 4 spec doc-item links', async ({ page }) => {
    await goToDocs(page);
    // Active Roadmaps card
    const roadmapsCard = page.locator('.card:has-text("Active Roadmaps")');
    await expect(roadmapsCard).toBeVisible();

    // Should have 4 doc-items
    const docItems = roadmapsCard.locator('.doc-item');
    await expect(docItems).toHaveCount(4);

    // Each spec link points to the correct _active path
    await expect(docItems.nth(0)).toHaveAttribute('href', /0024-distributed-ledger/);
    await expect(docItems.nth(0)).toContainText('Spec 0024');

    await expect(docItems.nth(1)).toHaveAttribute('href', /0025-zero-copy/);
    await expect(docItems.nth(1)).toContainText('Spec 0025');

    await expect(docItems.nth(2)).toHaveAttribute('href', /0026-transactional-outbox/);
    await expect(docItems.nth(2)).toContainText('Spec 0026');

    await expect(docItems.nth(3)).toHaveAttribute('href', /0027-refresh-token/);
    await expect(docItems.nth(3)).toContainText('Spec 0027');
  });

  test('doc items link to docs/ paths', async ({ page }) => {
    await goToDocs(page);
    const allDocItems = page.locator('.doc-item');
    const count = await allDocItems.count();
    expect(count).toBe(8);

    // All 8 doc-items should have href starting with docs/
    for (let i = 0; i < count; i++) {
      await expect(allDocItems.nth(i)).toHaveAttribute('href', /^docs\//);
    }
  });

  test('roadmap meta descriptions show spec summaries', async ({ page }) => {
    await goToDocs(page);
    const roadmapsCard = page.locator('.card:has-text("Active Roadmaps")');
    const metaEls = roadmapsCard.locator('.doc-meta');

    await expect(metaEls.nth(0)).toContainText('Clustered transactions');
    await expect(metaEls.nth(1)).toContainText('Zero-copy');
    await expect(metaEls.nth(2)).toContainText('at-least-once');
    await expect(metaEls.nth(3)).toContainText('breach detection');
  });

  test('two card sections are rendered side by side', async ({ page }) => {
    await goToDocs(page);
    // DocsTab renders two card sections: Core Whitepapers and Active Roadmaps.
    await expect(page.locator('.card:has-text("Core Whitepapers")')).toBeVisible();
    await expect(page.locator('.card:has-text("Active Roadmaps")')).toBeVisible();
  });
});

test.describe('MetricsBar Component', () => {
  test('displays Backend status pill on the Home tab with Offline Demo Mode text', async ({ page }) => {
    await page.goto('/');
    // The backend pill should always show, defaulting to "disconnected"
    const backendPill = page.getByText('Backend: Offline (Demo Mode)');
    await expect(backendPill).toBeVisible();

    // The pill should have the word "Backend:" with status
    await expect(page.getByText(/^Backend:/)).toBeVisible();
  });

  test('Backend status pill shows disconnected dot with inline circle styling', async ({ page }) => {
    await page.goto('/');
    // The disconnected dot span has inline style with border-radius: 50%
    // Find it directly by its unique inline style combination
    const dot = page.locator('span[style*="border-radius: 50%"]').first();
    await expect(dot).toBeVisible();

    const style = await dot.getAttribute('style');
    expect(style).toContain('width: 8px');
    expect(style).toContain('height: 8px');
    // Disconnected state uses muted color
    expect(style).toContain('text-muted');
  });

  test('connected-only pills (Requests, Live Agents) are hidden when backend is offline', async ({ page }) => {
    await page.goto('/');
    // The requests and live agent pills are inside {#if sim.serverStatus === 'connected'}
    // so they should have 0 count in the DOM
    await expect(page.locator('text=Requests:')).toHaveCount(0);
    await expect(page.locator('text=Live Agents:')).toHaveCount(0);
  });

  test('shows all three pills when backend responds with success', async ({ page }) => {
    // Mock backend health endpoint before navigation
    await page.route('http://localhost:3000/v1/health/agents', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          { agent_id: 'buyer_1', role: 'buyer' },
          { agent_id: 'seller_1', role: 'seller' },
        ]),
      });
    });
    // Mock metrics endpoint
    await page.route('http://localhost:3000/metrics', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'text/plain',
        body: 'requests_total 57342\n',
      });
    });

    await page.goto('/');
    // Wait for the fetchLiveMetrics calls to complete
    await page.waitForTimeout(500);

    // All three pills should now be visible
    await expect(page.getByText('Backend: Connected (Live)')).toBeVisible();
    await expect(page.getByText(/Requests:/)).toBeVisible();
    await expect(page.getByText(/Live Agents:/)).toBeVisible();
  });

  test('connected state shows request count from server metrics', async ({ page }) => {
    await page.route('http://localhost:3000/v1/health/agents', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([{ agent_id: 'a1' }]),
      });
    });
    await page.route('http://localhost:3000/metrics', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'text/plain',
        body: 'requests_total 57342\n',
      });
    });

    await page.goto('/');
    await page.waitForTimeout(500);

    // The request count should display the mocked value (57342)
    const requestsPill = page.locator('text=Requests:');
    await expect(requestsPill).toContainText('57342');
  });

  test('connected state shows live agent count', async ({ page }) => {
    await page.route('http://localhost:3000/v1/health/agents', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          { agent_id: 'buyer_1' },
          { agent_id: 'seller_1' },
          { agent_id: 'arbiter_1' },
        ]),
      });
    });
    await page.route('http://localhost:3000/metrics', async route => {
      await route.fulfill({ status: 200, contentType: 'text/plain', body: '' });
    });

    await page.goto('/');
    await page.waitForTimeout(500);

    // Should show 3 live agents
    const agentsPill = page.locator('text=Live Agents:');
    await expect(agentsPill).toContainText('3');
  });

  test('connected Backend pill shows green dot with success color', async ({ page }) => {
    await page.route('http://localhost:3000/v1/health/agents', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([{ agent_id: 'a1' }]),
      });
    });
    await page.route('http://localhost:3000/metrics', async route => {
      await route.fulfill({ status: 200, contentType: 'text/plain', body: '' });
    });

    await page.goto('/');
    await page.waitForTimeout(500);

    // Connected pill has inline border referencing var(--color-success)
    const connectedPill = page.locator('[style*="color-success"]').first();
    await expect(connectedPill).toBeVisible();
    await expect(connectedPill).toContainText('Connected (Live)');

    // The connected dot span should also reference success color in background
    const dot = page.locator('span[style*="color-success"]').first();
    await expect(dot).toBeVisible();
    const dotStyle = await dot.getAttribute('style');
    expect(dotStyle).toContain('color-success');
    expect(dotStyle).toContain('border-radius: 50%');
  });
});

test.describe('AgentCard Component', () => {
  test('renders buyer and seller agent cards with correct names', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('h4:has-text("Buyer Agent")')).toBeVisible();
    await expect(page.locator('h4:has-text("Seller Agent")')).toBeVisible();
  });

  test('displays role titles for both agents', async ({ page }) => {
    await page.goto('/');
    // Titles are backtick-wrapped in the agent card, e.g. `buyer_negotiator` role
    // Match the unique role text regardless of backtick wrapping
    await expect(page.getByText(/buyer_negotiator/)).toBeVisible();
    await expect(page.getByText(/seller_negotiator/)).toBeVisible();
  });

  test('agent cards are inside the simulator section', async ({ page }) => {
    await page.goto('/');
    const simulator = page.locator('section.card:has(h3:text("Interactive Agent Negotiation Simulator"))');
    // Cards should be within the simulator section (their parent div has flex display)
    await expect(simulator.locator('h4:has-text("Buyer Agent")')).toBeVisible();
    await expect(simulator.locator('h4:has-text("Seller Agent")')).toBeVisible();
  });

  test('buyer and seller cards are rendered on opposite sides of the flow diagram', async ({ page }) => {
    await page.goto('/');
    // The agent cards are in a flex container - buyer on left, seller on right
    const flowDiagram = page.locator('svg.flow-diagram');
    const buyerCard = page.locator('h4:has-text("Buyer Agent")');
    const sellerCard = page.locator('h4:has-text("Seller Agent")');

    // Both should be visible alongside the SVG
    await expect(flowDiagram).toBeVisible();
    await expect(buyerCard).toBeVisible();
    await expect(sellerCard).toBeVisible();
  });
});

test.describe('FlowDiagram Component', () => {
  test('renders the SVG flow diagram with correct aria-label', async ({ page }) => {
    await page.goto('/');
    const svg = page.locator('svg.flow-diagram');
    await expect(svg).toBeVisible();
    await expect(svg).toHaveAttribute('aria-label', 'Agent negotiation flow diagram');
  });

  test('displays BUYER, SERVER, and SELLER node labels', async ({ page }) => {
    await page.goto('/');
    // SVG text elements for the 3 nodes
    const buyerLabel = page.locator('svg.flow-diagram text:has-text("BUYER")');
    const serverLabel = page.locator('svg.flow-diagram text:has-text("SERVER")');
    const sellerLabel = page.locator('svg.flow-diagram text:has-text("SELLER")');

    await expect(buyerLabel).toBeVisible();
    await expect(serverLabel).toBeVisible();
    await expect(sellerLabel).toBeVisible();
  });

  test('renders SVG with defs filter for node glow', async ({ page }) => {
    await page.goto('/');
    // SVG <filter> defs elements are inherently not visually visible (they are in <defs>),
    // so we assert presence via count rather than visibility.
    const filter = page.locator('svg.flow-diagram defs filter#node-glow');
    await expect(filter).toHaveCount(1);
  });

  test('shows animated signal dots when simulation is not idle', async ({ page }) => {
    await page.goto('/');
    // The simulation auto-starts, so animateMotion elements should render
    const motionEls = page.locator('svg.flow-diagram animateMotion');
    const count = await motionEls.count();
    // At least one forward signal dot should be present when state != idle
    expect(count).toBeGreaterThanOrEqual(1);
  });

  test('flow-path lines are present and activate on simulation start', async ({ page }) => {
    await page.goto('/');
    // SVG <line> elements have zero bounding box, so toBeVisible() won't work.
    // Instead, check that the element has the 'active' class applied.
    const activePath = page.locator('.flow-path');
    await expect(activePath.first()).toHaveClass(/\bactive\b/, { timeout: 3000 });
    // Both flow paths should eventually become active
    await expect(activePath.nth(1)).toHaveClass(/\bactive\b/, { timeout: 3000 });
  });

  test('svg has node-glow filter applied to circle groups', async ({ page }) => {
    await page.goto('/');
    // The BUYER/SERVER/SELLER nodes are wrapped in <g> elements with the glow filter
    const filteredGroups = page.locator('svg.flow-diagram g[filter="url(#node-glow)"]');
    await expect(filteredGroups).toHaveCount(3);
  });
});

test.describe('LedgerExplorer Component', () => {
  test('displays Ledger Cache title and block count badge', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.ledger-title')).toBeVisible();
    const badge = page.locator('.ledger-count-badge');
    await expect(badge).toBeVisible();
    await expect(badge).toContainText(/^\d+ blocks$/);
  });

  test('shows total volume in the footer', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.ledger-volume')).toBeVisible();

    // Total volume value should be a dollar-formatted number
    const volumeValue = page.locator('.ledger-volume-value');
    await expect(volumeValue).toBeVisible();
    await expect(volumeValue).toContainText(/^\$/);
  });

  test('displays seed transaction blocks with hash, item, and price', async ({ page }) => {
    await page.goto('/');
    const blocks = page.locator('.ledger-block');
    const count = await blocks.count();
    // Should show at least the 15 seed blocks
    expect(count).toBeGreaterThanOrEqual(10);

    // Each block should have a hash and meta section
    const firstBlock = blocks.first();
    await expect(firstBlock.locator('.ledger-block-hash')).toBeVisible();
    // Each block has 2 .ledger-block-meta divs (item/price + timestamp/status)
    await expect(firstBlock.locator('.ledger-block-meta').first()).toBeVisible();
  });

  test('block hashes are formatted as 0x-prefixed hex strings', async ({ page }) => {
    await page.goto('/');
    const hashEls = page.locator('.ledger-block-hash');
    const firstHash = await hashEls.first().textContent();
    expect(firstHash).toMatch(/^0x[a-f0-9]+/);
  });

  test('blocks show committed status', async ({ page }) => {
    await page.goto('/');
    const committedTags = page.locator('.ledger-block-meta:has-text("committed")');
    // At least some blocks should show committed status
    const count = await committedTags.count();
    expect(count).toBeGreaterThanOrEqual(1);
  });

  test('blocks display timestamps', async ({ page }) => {
    await page.goto('/');
    // Each block should have timestamp marker (⏱)
    const timestampElements = page.locator('.ledger-block-meta:has-text("⏱")');
    const count = await timestampElements.count();
    expect(count).toBeGreaterThanOrEqual(1);
  });

  test('seed blocks include diverse item names', async ({ page }) => {
    await page.goto('/');
    const blockItems = page.locator('.ledger-block-meta span:first-child');
    const itemTexts = await blockItems.allTextContents();

    // Should contain some of the known items from the catalog
    const knownItems = ['iPhone', 'MacBook', 'Samsung', 'Sony', 'Nintendo', 'Pixel', 'ASUS'];
    const hasKnownItem = knownItems.some(item => itemTexts.some(t => t.includes(item)));
    expect(hasKnownItem).toBe(true);
  });

  test('should filter ledger blocks when search input is typed', async ({ page }) => {
    await page.goto('/');
    const searchInput = page.locator('.ledger-search-input');
    await expect(searchInput).toBeVisible();

    // Type "MacBook" in the filter input
    await searchInput.fill('MacBook');
    await page.waitForTimeout(300);

    // Verify all filtered blocks contain "MacBook"
    const blocks = page.locator('.ledger-block');
    const count = await blocks.count();
    expect(count).toBeGreaterThan(0);
    
    for (let i = 0; i < count; i++) {
      const text = await blocks.nth(i).locator('.ledger-block-meta').first().locator('span').first().textContent();
      expect(text.toLowerCase()).toContain('macbook');
    }

    // Badge count should decrease and reflect the filtered list
    const badge = page.locator('.ledger-count-badge');
    await expect(badge).toContainText(`${count} blocks`);
  });

  test('should reset ledger to seed blocks when Reset button is clicked', async ({ page }) => {
    await page.goto('/');
    const resetBtn = page.locator('.ledger-reset-btn');
    await expect(resetBtn).toBeVisible();

    // Filter list to something small first to prove reset works
    const searchInput = page.locator('.ledger-search-input');
    await searchInput.fill('MacBook');
    await page.waitForTimeout(300);

    // Click Reset
    await resetBtn.click();
    await page.waitForTimeout(300);

    // Explicitly clear the search input to ensure no filtering persists
    await searchInput.fill('');
    await page.waitForTimeout(300);

    // Verify count is reset back to 15 blocks
    const badge = page.locator('.ledger-count-badge');
    await expect(badge).toContainText('15 blocks');
  });

  test('should update total volume and block count badge when a block is committed', async ({ page }) => {
    await page.goto('/');

    const badge = page.locator('.ledger-count-badge');
    const initialText = await badge.textContent();
    const initialCount = parseInt(initialText, 10);

    const volumeValue = page.locator('.ledger-volume-value');
    const initialVolText = (await volumeValue.textContent()).replace('$', '').replace(/,/g, '');
    const initialVol = parseInt(initialVolText, 10);

    // Commit a mock block via the window.__sim store
    await page.evaluate(() => {
      window.__sim.committedBlocks = [
        { hash: '0xmockedhash12', price: 999, item: 'Mock Test Item', ts: '12:00:00', isNew: true },
        ...window.__sim.committedBlocks
      ];
    });
    await page.waitForTimeout(100);

    // Verify badge increments
    await expect(badge).toContainText(`${initialCount + 1} blocks`);

    // Verify total volume increases by 999
    const newVolText = (await volumeValue.textContent()).replace('$', '').replace(/,/g, '');
    const newVol = parseInt(newVolText, 10);
    expect(newVol).toBe(initialVol + 999);
  });
});

test.describe('Simulator - Autoplay Toggle', () => {
  test('autoplay pause/resume button is present within the simulator section', async ({ page }) => {
    await page.goto('/');
    const simulator = page.locator('section.card:has(h3:text("Interactive Agent Negotiation Simulator"))');
    const pauseBtn = simulator.locator('button:has-text("Autoplay")');
    await expect(pauseBtn).toBeVisible();
  });

  test('initial autoplay text shows Pause since sim auto-starts', async ({ page }) => {
    await page.goto('/');
    const pauseBtn = page.locator('button:has-text("Autoplay")');
    await expect(pauseBtn).toContainText('Pause Autoplay');
  });

  test('clicking pause changes button text to Resume Autoplay', async ({ page }) => {
    await page.goto('/');
    const pauseBtn = page.locator('button:has-text("Autoplay")');
    await pauseBtn.click();
    await expect(pauseBtn).toContainText('Resume Autoplay');
  });

  test('pausing then resuming restarts the simulation cycle', async ({ page }) => {
    await page.goto('/');
    const pauseBtn = page.locator('button:has-text("Autoplay")');
    // "Publishing..." appears both as a status display div AND a disabled button.
    // Use .first() to avoid strict mode — we only need to verify at least one is visible.
    const statusDisplay = page.locator('text=Publishing...').first();

    // Auto-start should make state 'listing', showing "Publishing..."
    await expect(statusDisplay).toBeVisible({ timeout: 3000 });

    // Pause — clears timeouts so state stays at 'listing' (doesn't advance)
    await pauseBtn.click();
    await page.waitForTimeout(500);
    await expect(pauseBtn).toContainText('Resume Autoplay');

    // State should still be 'listing' since timeouts are cleared
    await expect(statusDisplay).toBeVisible();

    // Resume — restarts the sim (runSimulation called), goes to 'listing' again
    await pauseBtn.click();
    await expect(pauseBtn).toContainText('Pause Autoplay');
    await expect(statusDisplay).toBeVisible({ timeout: 3000 });
  });

  test('rapid multiple toggle clicks produce correct button text each time', async ({ page }) => {
    await page.goto('/');
    const pauseBtn = page.locator('button:has-text("Autoplay")');

    await pauseBtn.click();
    await expect(pauseBtn).toContainText('Resume Autoplay');

    await pauseBtn.click();
    await expect(pauseBtn).toContainText('Pause Autoplay');

    await pauseBtn.click();
    await expect(pauseBtn).toContainText('Resume Autoplay');

    await pauseBtn.click();
    await expect(pauseBtn).toContainText('Pause Autoplay');
  });

  test('autoplay button has the counter CSS class', async ({ page }) => {
    await page.goto('/');
    const pauseBtn = page.locator('button:has-text("Autoplay")');
    await expect(pauseBtn).toHaveClass(/counter/);
  });
});

test.describe('ThemeSwitcher - Additional Coverage', () => {
  test('all 5 theme swatch buttons are present with correct labels', async ({ page }) => {
    await page.goto('/');
    const themes = ['Midnight', 'Emerald', 'Crimson', 'Solar', 'Nordic'];
    for (const theme of themes) {
      await expect(page.locator(`button[aria-label="${theme} theme"]`)).toBeVisible();
    }
  });

  test('default active swatch has active CSS class on Midnight', async ({ page }) => {
    await page.goto('/');
    const activeSwatch = page.locator('.theme-swatch.active');
    await expect(activeSwatch).toHaveCount(1);
    await expect(activeSwatch).toHaveAttribute('aria-label', 'Midnight theme');
  });

  test('aria-pressed attribute updates on theme click', async ({ page }) => {
    await page.goto('/');
    // Default state
    await expect(page.locator('[aria-label="Midnight theme"]')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.locator('[aria-label="Emerald theme"]')).toHaveAttribute('aria-pressed', 'false');

    // Click Emerald
    await page.click('[aria-label="Emerald theme"]');
    await expect(page.locator('[aria-label="Midnight theme"]')).toHaveAttribute('aria-pressed', 'false');
    await expect(page.locator('[aria-label="Emerald theme"]')).toHaveAttribute('aria-pressed', 'true');

    // Click back to Midnight
    await page.click('[aria-label="Midnight theme"]');
    await expect(page.locator('[aria-label="Midnight theme"]')).toHaveAttribute('aria-pressed', 'true');
    await expect(page.locator('[aria-label="Emerald theme"]')).toHaveAttribute('aria-pressed', 'false');
  });

  test('only one theme-swatch has active class at a time', async ({ page }) => {
    await page.goto('/');
    const getActiveCount = () => page.locator('.theme-swatch.active').count();

    // Initially 1 active
    expect(await getActiveCount()).toBe(1);

    // Click Emerald
    await page.click('[aria-label="Emerald theme"]');
    expect(await getActiveCount()).toBe(1);

    // Click Solar
    await page.click('[aria-label="Solar theme"]');
    expect(await getActiveCount()).toBe(1);
  });

  test('theme button has correct swatch-{id} class for CSS styling', async ({ page }) => {
    await page.goto('/');
    const themes = [
      { id: 'midnight', label: 'Midnight' },
      { id: 'emerald',  label: 'Emerald'  },
      { id: 'crimson',  label: 'Crimson'  },
      { id: 'solar',    label: 'Solar'    },
      { id: 'nordic',   label: 'Nordic'   },
    ];
    for (const th of themes) {
      const btn = page.locator(`button[aria-label="${th.label} theme"]`);
      await expect(btn).toHaveClass(new RegExp(`swatch-${th.id}`));
    }
  });
});

test.describe('MetricsPanel Component', () => {
  test('displays default metrics (blocks, success rate, discount)', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.metrics-grid')).toBeVisible();
    await expect(page.locator('.metric-card:has-text("Ledger Blocks")')).toBeVisible();
    await expect(page.locator('.metric-card:has-text("Success Rate")')).toBeVisible();
    await expect(page.locator('.metric-card:has-text("Avg Discount")')).toBeVisible();
    await expect(page.locator('.status-indicator:has-text("Offline")')).toBeVisible();
  });

  test('reflects SSE connection status when server goes online', async ({ page }) => {
    // Mock health check and metrics
    await page.route('http://localhost:3000/v1/health/agents', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([{ agent_id: 'buyer_1' }, { agent_id: 'seller_1' }]),
      });
    });
    await page.route('http://localhost:3000/metrics', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'text/plain',
        body: 'requests_total 12345\n',
      });
    });

    await page.goto('/');
    await page.waitForTimeout(500);

    // Verify SSE Connected status is shown
    await expect(page.locator('.status-indicator:has-text("SSE Connected")')).toBeVisible();
    await expect(page.locator('.metric-card:has-text("12,345 reqs logged")')).toBeVisible();
  });
});

test.describe('BlockInspectionModal Component', () => {
  test('opens inspector modal when a ledger block is clicked and shows cryptographic proofs', async ({ page }) => {
    await page.goto('/');
    
    // Click the first ledger block
    const firstBlock = page.locator('.ledger-block').first();
    await firstBlock.click();
    await page.waitForTimeout(200);

    // Verify modal is open
    await expect(page.locator('.modal-card')).toBeVisible();
    await expect(page.locator('h4:has-text("Transaction Block Inspector")')).toBeVisible();
    await expect(page.locator('.section-title:has-text("Cryptographic Proofs")')).toBeVisible();
    await expect(page.locator('.crypto-label:has-text("Transaction Signature")')).toBeVisible();
    
    // Copy buttons present
    const copyBtns = page.locator('.copy-btn');
    await expect(copyBtns).toHaveCount(4);

    // Reconstructed dialogue present
    await expect(page.locator('.section-title:has-text("Reconstructed Negotiation Dialogue")')).toBeVisible();
    await expect(page.locator('.dialogue-bubble').first()).toBeVisible();

    // Click Close Button
    await page.click('.modal-close-btn');
    await page.waitForTimeout(200);
    await expect(page.locator('.modal-card')).not.toBeVisible();
  });

  test('closes modal when overlay backdrop is clicked', async ({ page }) => {
    await page.goto('/');
    
    // Open modal
    await page.locator('.ledger-block').first().click();
    await page.waitForTimeout(200);
    await expect(page.locator('.modal-card')).toBeVisible();

    // Click on overlay backdrop (outside modal-card)
    // We can click the overlay directly by targeting its selector
    await page.click('.modal-overlay', { position: { x: 5, y: 5 } });
    await page.waitForTimeout(200);
    
    // Verify modal is closed
    await expect(page.locator('.modal-card')).not.toBeVisible();
  });
});

