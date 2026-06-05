<script>
  import ThemeSwitcher from './lib/ThemeSwitcher.svelte';
  import BackgroundSwitcher from './lib/BackgroundSwitcher.svelte';
  import MetricsBar from './lib/MetricsBar.svelte';
  import Simulator from './lib/Simulator.svelte';
  import GuideTab from './lib/GuideTab.svelte';
  import DocsTab from './lib/DocsTab.svelte';
  import FAQTab from './lib/FAQTab.svelte';
  import StatusTab from './lib/StatusTab.svelte';
  import { untrack, onMount } from 'svelte';
  import {
    sim,
    runSimulation,
    fetchLiveMetrics,
    clearAllTimeouts,
  } from './lib/simulator.svelte.js';

  const validPlatforms = ['web', 'mcp', 'android', 'ios'];

  let currentTab = $state('home');
  let platformTab = $state('web');

  function parsePath(path) {
    path = path.replace(/\/+$/, '') || '/';
    if (path === '/') return { tab: 'home', platform: 'web' };
    const parts = path.split('/').filter(Boolean);
    if (parts[0] === 'getting-started') {
      const p = parts[1] || 'web';
      return { tab: 'guide', platform: validPlatforms.includes(p) ? p : 'web' };
    }
    if (parts[0] === 'faqs') return { tab: 'faqs', platform: 'web' };
    if (parts[0] === 'status') return { tab: 'status', platform: 'web' };
    if (parts[0] === 'docs') return { tab: 'docs', platform: 'web' };
    if (parts[0] !== '') return { tab: 'not-found', platform: 'web' };
    return { tab: 'home', platform: 'web' };
  }

  function buildPath(tab, platform) {
    if (tab === 'home') return '/';
    if (tab === 'guide') {
      const p = platform || 'web';
      return p === 'web' ? '/getting-started' : `/getting-started/${p}`;
    }
    if (tab === 'faqs') return '/faqs';
    if (tab === 'status') return '/status';
    if (tab === 'docs') return '/docs';
    return '/';
  }

  function navigate(tab, platform) {
    const path = buildPath(tab, platform);
    currentTab = tab;
    platformTab = platform || 'web';
    history.pushState({ tab, platform }, '', path);
  }

  onMount(() => {
    const r = parsePath(window.location.pathname);
    currentTab = r.tab;
    platformTab = r.platform;

    window.addEventListener('popstate', (e) => {
      const r2 = parsePath(window.location.pathname);
      currentTab = r2.tab;
      platformTab = r2.platform;
    });
  });

  // Simulation runner — re-runs when sim.isPaused changes.
  // CRITICAL: wrap runSimulation in untrack() so its internal $state reads/writes
  // do NOT register as dependencies — otherwise every state change triggers
  // a re-run, creating an infinite effect_update_depth_exceeded loop.
  $effect(() => {
    const paused = sim.isPaused;

    untrack(() => {
      if (!paused) {
        runSimulation();
      }

      return () => {
        clearAllTimeouts();
      };
    });
  });

  // Metrics polling — runs once on mount, independent of pause state.
  $effect(() => {
    fetchLiveMetrics();
    const interval = setInterval(fetchLiveMetrics, 3000);

    return () => {
      clearInterval(interval);
    };
  });
</script>

<style>
  /* ── Header & Nav ── */
  header {
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    background: var(--bg-dark);
    width: 100%;
  }

  .header-inner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem 2rem;
    width: min(88%, 1400px);
    margin-inline: auto;
  }

  .logo-container {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .logo-container h1 {
    font-family: var(--font-heading);
    font-size: 1.5rem;
    font-weight: 800;
    letter-spacing: -0.5px;
    background: linear-gradient(135deg, var(--text-primary) 30%, var(--color-primary) 70%, var(--color-secondary));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  .wip-badge {
    font-family: var(--font-mono);
    font-size: 0.6rem;
    font-weight: 700;
    color: var(--color-primary);
    background: var(--color-primary-glow);
    border: 1px solid var(--color-primary);
    padding: 0.15rem 0.45rem;
    border-radius: 4px;
    letter-spacing: 0.5px;
    line-height: 1;
  }

  nav {
    display: flex;
    gap: 0.35rem;
    background: rgba(0, 0, 0, 0.25);
    padding: 0.35rem;
    border-radius: 30px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  nav button {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-family: var(--font-sans);
    font-weight: 600;
    font-size: 0.85rem;
    padding: 0.5rem 0.9rem;
    border-radius: 20px;
    cursor: pointer;
    white-space: nowrap;
  }

  nav button.active {
    background: var(--color-primary);
    color: var(--text-primary);
    box-shadow: 0 4px 14px rgba(170, 59, 255, 0.3), inset 0 -1.5px 0 0 rgba(255, 255, 255, 0.25);
  }

  .header-right {
    display: flex;
    flex-direction: column;
    align-items: end;
    gap: 0.5rem;
  }

  .header-right-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .auth-btn {
    font-family: var(--font-sans);
    font-weight: 600;
    font-size: 0.85rem;
    padding: 0.5rem 1rem;
    border-radius: 20px;
    cursor: pointer;
    border: none;
    transition: background-color 0.2s ease, color 0.2s ease;
  }

  .auth-btn.signin {
    background: transparent;
    color: var(--text-secondary);
  }

  .auth-btn.signin:hover {
    color: var(--text-primary);
  }

  .auth-btn.signup {
    background: var(--color-primary);
    color: var(--text-primary);
  }

  .auth-btn.signup:hover {
    box-shadow: 0 4px 14px rgba(170, 59, 255, 0.3);
  }

  .anchor-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-primary);
    font-weight: 600;
    font-size: inherit;
    font-family: inherit;
    text-decoration: underline;
    padding: 0;
  }

  .anchor-btn:hover {
    color: var(--color-secondary);
  }

  /* ── Hero & Badge ── */
  .hero {
    text-align: center;
    padding: 4rem 1.5rem 2rem;
    max-width: 900px;
    margin-inline: auto;
  }

  .hero h2 {
    font-family: var(--font-heading);
    font-size: 3rem;
    font-weight: 800;
    line-height: 1.15;
    margin-bottom: 1.5rem;
    letter-spacing: -1px;
  }

  .hero h2 span {
    background: linear-gradient(90deg, var(--color-primary), var(--color-secondary));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  .hero p {
    color: var(--text-secondary);
    font-size: 1.15rem;
    margin-bottom: 2rem;
    max-width: 600px;
    margin-left: auto;
    margin-right: auto;
  }

  .badge {
    display: inline-block;
    background: var(--color-primary-glow);
    border: 1px solid var(--border-glow-hover);
    color: var(--color-primary);
    padding: 0.4rem 1rem;
    border-radius: 30px;
    font-size: 0.8rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 1.5rem;
  }

  /* ── Layout Grid & Cards ── */
  .container {
    width: min(88%, 1400px);
    margin-inline: auto;
    padding: 2rem 1.5rem 4rem;
  }

  .grid-3 {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 1.5rem;
    margin-bottom: 3rem;
  }

  /* ── Pulse Animation ── */
  .pulse-glow {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-secondary);
    display: inline-block;
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(0, 242, 254, 0.7); }
    70% { transform: scale(1); box-shadow: 0 0 0 10px rgba(0, 242, 254, 0); }
    100% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(0, 242, 254, 0); }
  }

  /* ── Benchmark Table ── */
  .table-container {
    overflow-x: auto;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    margin-top: 1.5rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
    background: var(--bg-card);
    font-size: 0.9rem;
  }

  th, td {
    padding: 1rem 1.25rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  th {
    background: rgba(255, 255, 255, 0.03);
    font-family: var(--font-heading);
    font-weight: 600;
    color: var(--text-primary);
  }

  tr:last-child td {
    border-bottom: none;
  }

  tr:hover td {
    background: var(--color-primary-glow);
  }

  /* ── Footer ── */
  footer {
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    color: var(--text-muted);
    font-size: 0.85rem;
    background: var(--bg-dark);
  }

  .footer-inner {
    width: min(88%, 1400px);
    margin-inline: auto;
    padding: 1.5rem 0 1.5rem;
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 2rem;
    align-items: start;
  }

  .footer-brand h3 {
    font-family: var(--font-heading);
    font-size: 1.15rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 0.5rem;
  }

  .footer-brand p {
    color: var(--text-muted);
    font-size: 0.8rem;
    line-height: 1.5;
    max-width: 220px;
  }

  .footer-links {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .footer-links h4 {
    font-family: var(--font-heading);
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 0.25rem;
  }

  .footer-links button {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
    font-size: 0.85rem;
    text-align: left;
    padding: 0;
    transition: color 0.2s ease;
  }

  .footer-links button:hover {
    color: var(--color-primary);
  }

    .footer-social {
      display: flex;
      flex-direction: column;
      gap: 0.75rem;
      justify-self: end;
      align-items: center;
    }

    .footer-social .social-icons {
      display: flex;
      gap: 1rem;
      align-items: center;
    }

    .footer-copy-bar {
      text-align: right;
      padding: 0 0 0.75rem;
      width: min(88%, 1400px);
      margin-inline: auto;
      color: var(--text-muted);
      font-size: 0.64rem;
    }

  .footer-social button {
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    color: var(--text-muted);
    transition: color 0.2s ease, transform 0.2s ease;
  }

  .footer-social button:hover {
    color: var(--color-primary);
    transform: translateY(-2px);
  }

  .footer-social button:active {
    transform: scale(0.9);
  }

  .footer-social svg {
    width: 22px;
    height: 22px;
    display: block;
  }

  @media (max-width: 500px) {
    .hero h2 {
      font-size: 2rem;
    }
    .hero p {
      font-size: 1rem;
    }
  }

  @media (max-width: 700px) {
    .footer-inner {
      grid-template-columns: 1fr;
      text-align: center;
      gap: 2rem;
    }
    .footer-brand p {
      max-width: none;
    }
    .footer-social {
      justify-self: center;
    }
    .footer-social .social-icons {
      justify-content: center;
    }
    .footer-copy-bar {
      text-align: center;
    }
  }

  /* ── Responsive: header/nav wraps ── */
  @media (max-width: 600px) {
    header {
      display: grid;
      grid-template-columns: 1fr auto;
      gap: 1.25rem;
      padding: 1.25rem 1.5rem;
      align-items: center;
    }
    nav {
      grid-column: span 2;
      justify-content: space-around;
      width: 100%;
    }
  }

  @media (max-width: 400px) {
    header {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.75rem;
      padding: 1rem;
    }
    nav {
      width: 100%;
      justify-content: space-around;
    }
    nav button {
      padding: 0.5rem 0.8rem;
      font-size: 0.8rem;
    }
    .header-right {
      align-items: center;
    }
  }
</style>

<header>
  <div class="header-inner">
    <div class="logo-container">
      <span class="pulse-glow"></span>
      <h1>oz-market</h1>
      <span class="wip-badge">WIP</span>
    </div>
    <nav>
      <button class={currentTab === 'home' ? 'active' : ''} onclick={() => navigate('home')}>Home</button>
      <button class={currentTab === 'guide' ? 'active' : ''} onclick={() => navigate('guide', platformTab)}>Getting Started</button>
      <button class={currentTab === 'faqs' ? 'active' : ''} onclick={() => navigate('faqs')}>FAQs</button>
      <button class={currentTab === 'status' ? 'active' : ''} onclick={() => navigate('status')}>Status</button>
      <button class={currentTab === 'docs' ? 'active' : ''} onclick={() => navigate('docs')}>Docs</button>
    </nav>
    <div class="header-right">
      <div class="header-right-row">
        <ThemeSwitcher />
        <button type="button" class="auth-btn signin" onclick={() => alert('Sign in flow — coming soon.')}>Sign In</button>
        <button type="button" class="auth-btn signup" onclick={() => alert('Sign up flow — coming soon.')}>Sign Up</button>
      </div>
      <BackgroundSwitcher />
    </div>
  </div>
</header>

<main class="container">
  {#if currentTab === 'home'}
    <!-- Home Tab -->
    <section class="hero">
      <span class="badge">Next-Gen Agentic Commerce</span>
      <h2>Autonomous <span>AI-to-AI</span> Commerce Infrastructure</h2>
      <p>
        A decentralized marketplace built in Rust — AI agents search listings, negotiate deals, and settle transactions autonomously with encrypted privacy and sub-millisecond speed.
      </p>
    </section>

    <!-- Live Server Stats Banner -->
    <MetricsBar />

    <!-- Interactive Agent Simulator -->
    <Simulator />

    <!-- Value Pillars -->
    <div class="grid-3">
      <div class="card">
        <h3><span>⚡</span> High-Frequency Scale</h3>
        <p>Built using Actix-web and optimized async Rust. Sustains over 57,000 requests per second under concurrent load testing with sub-millisecond route resolution.</p>
      </div>
      <div class="card">
        <h3><span>🔒</span> Zero-Knowledge Privacy</h3>
        <p>Buyer agents browse listings publicly, but negotiate anonymously. Direct seller contact details are kept strictly encrypted until binding consensus is achieved.</p>
      </div>
      <div class="card">
        <h3><span>📊</span> Dual-Layer Ledger</h3>
        <p>In-memory DashMap ledger cache with write-through PostgreSQL replication. Powers sub-ms real-time credits checks and balance updates safely.</p>
      </div>
    </div>

    <!-- Benchmark Baselines -->
    <section class="card" style="margin-bottom: 3rem;">
      <h3><span>📈</span> Core Performance Benchmarks</h3>
      <p>Simulating concurrent agent search lookups against a local PostgreSQL database with active rate-limiting diagnostics.</p>

      <div class="table-container">
        <table>
          <thead>
            <tr>
              <th>Search Concurrency</th>
              <th>Throughput (Public Search)</th>
              <th>Throughput (Rotating Auth)</th>
              <th>Rate Limit (429) Rejection</th>
              <th>Avg Latency</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td><strong>100 concurrent</strong></td>
              <td>57,733 ops/s</td>
              <td>57,418 ops/s</td>
              <td>0%</td>
              <td>&lt; 1.8ms</td>
            </tr>
            <tr>
              <td><strong>200 concurrent</strong></td>
              <td>57,350 ops/s</td>
              <td>59,140 ops/s</td>
              <td>0%</td>
              <td>&lt; 3.4ms</td>
            </tr>
            <tr>
              <td><strong>500 concurrent</strong></td>
              <td>51,569 ops/s</td>
              <td>47,946 ops/s</td>
              <td>0%</td>
              <td>&lt; 9.7ms</td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

  {:else if currentTab === 'guide'}
    <GuideTab {platformTab} onTabChange={(p) => navigate('guide', p)} />

  {:else if currentTab === 'faqs'}
    <FAQTab />

  {:else if currentTab === 'status'}
    <StatusTab />

  {:else if currentTab === 'docs'}
    <DocsTab />

  {:else if currentTab === 'not-found'}
    <section class="hero" style="padding-top: 4rem;">
      <h2 style="font-size: 5rem; margin-bottom: 0.5rem; opacity: 0.3;">404</h2>
      <p style="font-size: 1.2rem; max-width: 500px; margin-inline: auto;">Page not found. Check the URL or head back <button class="anchor-btn" onclick={() => navigate('home')}>home</button>.</p>
    </section>
  {/if}
</main>

<footer>
  <div class="footer-inner">
    <div class="footer-brand">
      <h3>oz-market</h3>
      <p>Autonomous AI-to-AI commerce infrastructure built in Rust.</p>
    </div>
    <div class="footer-links">
      <h4>Quick Links</h4>
      <button onclick={() => navigate('home')}>Home</button>
      <button onclick={() => navigate('guide', platformTab)}>Getting Started</button>
      <button onclick={() => navigate('faqs')}>FAQs</button>
      <button onclick={() => navigate('status')}>Status</button>
      <button onclick={() => navigate('docs')}>Docs</button>
    </div>
    <div class="footer-social">
      <div class="social-icons">
        <button type="button" aria-label="X / Twitter" onclick={() => alert('X/Twitter — coming soon.')}>
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"/>
          </svg>
        </button>
        <button type="button" aria-label="Discord" onclick={() => alert('Discord — coming soon.')}>
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path d="M20.317 4.37a19.791 19.791 0 00-4.885-1.515.074.074 0 00-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 00-5.487 0 12.64 12.64 0 00-.617-1.25.077.077 0 00-.079-.037A19.736 19.736 0 003.677 4.37a.07.07 0 00-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 00.031.057 19.9 19.9 0 005.993 3.03.078.078 0 00.084-.028 14.09 14.09 0 001.226-1.994.076.076 0 00-.041-.106 13.107 13.107 0 01-1.872-.892.077.077 0 01-.008-.128 10.2 10.2 0 00.372-.292.074.074 0 01.077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 01.078.01c.12.098.246.198.373.292a.077.077 0 01-.006.127 12.299 12.299 0 01-1.873.892.077.077 0 00-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 00.084.028 19.839 19.839 0 006.002-3.03.077.077 0 00.032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 00-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/>
          </svg>
        </button>
        <button type="button" aria-label="Telegram" onclick={() => alert('Telegram — coming soon.')}>
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path d="M11.944 0A12 12 0 000 12a12 12 0 0012 12 12 12 0 0012-12A12 12 0 0012 0a12 12 0 00-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 01.171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.386 4.025-1.627 4.476-1.635z"/>
          </svg>
        </button>
        <button type="button" aria-label="Medium" onclick={() => alert('Medium — coming soon.')}>
          <svg viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg" fill="currentColor">
            <path d="M4,4V28H28V4ZM23.9385,9.6865,22.6514,10.92a.3766.3766,0,0,0-.1431.3613v9.0674a.3765.3765,0,0,0,.1431.3613l1.257,1.2339v.271h-6.323v-.271L18.8877,20.68c.1279-.128.1279-.1656.1279-.3609V12.99l-3.62,9.1958H14.906L10.6907,12.99v6.1631a.8505.8505,0,0,0,.2334.7071l1.6936,2.0547v.2709H7.8154v-.2709L9.509,19.86a.82.82,0,0,0,.2183-.7071V12.0264A.6231.6231,0,0,0,9.5239,11.5L8.0186,9.6865v-.271h4.6743l3.613,7.9239,3.1765-7.9239h4.4561Z"></path>
          </svg>
        </button>
        <button type="button" aria-label="GitHub" onclick={() => window.open('https://github.com/kardelitaitu/oz-market', '_blank')}>
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/>
          </svg>
        </button>
      </div>
    </div>
  </div>
  <div class="footer-copy-bar">© 2026 oz-market. Built with Svelte 5 + Vite. Non-commercial use permissions apply.</div>
</footer>
