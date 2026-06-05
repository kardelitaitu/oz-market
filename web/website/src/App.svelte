<script>
  import ThemeSwitcher from './lib/ThemeSwitcher.svelte';
  import MetricsBar from './lib/MetricsBar.svelte';
  import Simulator from './lib/Simulator.svelte';
  import GuideTab from './lib/GuideTab.svelte';
  import DocsTab from './lib/DocsTab.svelte';
  import { untrack } from 'svelte';
  import {
    sim,
    runSimulation,
    fetchLiveMetrics,
    clearAllTimeouts,
  } from './lib/simulator.svelte.js';

  let currentTab = $state('home');

  // Auto-start: runs simulation + metrics polling on mount.
  // The $effect re-runs when sim.isPaused changes.
  // CRITICAL: wrap runSimulation in untrack() so its internal $state reads/writes
  // do NOT register as dependencies — otherwise every state change triggers
  // a re-run, creating an infinite effect_update_depth_exceeded loop.
  $effect(() => {
    // Track isPaused so the effect re-runs when paused state changes
    const paused = sim.isPaused;

    untrack(() => {
      if (!paused) {
        runSimulation();
      }

      fetchLiveMetrics();
      let interval = setInterval(fetchLiveMetrics, 3000);

      return () => {
        clearAllTimeouts();
        clearInterval(interval);
      };
    });
  });
</script>

<style>
  /* ── Header & Nav ── */
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem 2rem;
    max-width: 1200px;
    margin: 0 auto;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
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

  nav {
    display: flex;
    gap: 0.5rem;
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
    font-size: 0.9rem;
    padding: 0.6rem 1.25rem;
    border-radius: 20px;
    cursor: pointer;
  }

  nav button.active {
    background: var(--color-primary);
    color: var(--text-primary);
    box-shadow: 0 4px 14px rgba(170, 59, 255, 0.3);
  }

  /* ── Hero & Badge ── */
  .hero {
    text-align: center;
    padding: 4rem 1.5rem 2rem;
    max-width: 800px;
    margin: 0 auto;
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
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem 1.5rem 4rem;
  }

  .grid-3 {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 1.5rem;
    margin-bottom: 3rem;
  }

  .card {
    background: var(--bg-card);
    backdrop-filter: var(--glass-backdrop);
    border: 1px solid var(--border-glow);
    border-radius: 16px;
    padding: 2rem;
    box-shadow: var(--glass-shadow);
    display: flex;
    flex-direction: column;
    height: 100%;
    transition: background-color 0.3s ease, border-color 0.4s ease 0.1s, transform 0.2s ease, box-shadow 0.4s ease 0.1s;
  }

  .card:hover {
    background: var(--bg-card-hover);
    border-color: var(--border-glow-hover);
    transform: translateY(-4px);
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5), 0 0 15px var(--border-glow);
  }

  .card h3 {
    font-family: var(--font-heading);
    font-size: 1.35rem;
    margin-bottom: 1rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .card p {
    color: var(--text-secondary);
    font-size: 0.95rem;
    margin-bottom: 1.5rem;
    flex-grow: 1;
  }

  /* ── Pulse Animation ── */
  .pulse-glow {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-secondary);
    box-shadow: 0 0 10px var(--color-secondary);
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
    text-align: center;
    padding: 3rem 1.5rem;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    color: var(--text-muted);
    font-size: 0.85rem;
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
  }
</style>

<header>
  <div class="logo-container">
    <span class="pulse-glow"></span>
    <h1>oz-market</h1>
  </div>
  <nav>
    <button class={currentTab === 'home' ? 'active' : ''} onclick={() => currentTab = 'home'}>Home</button>
    <button class={currentTab === 'guide' ? 'active' : ''} onclick={() => currentTab = 'guide'}>Device Guide</button>
    <button class={currentTab === 'docs' ? 'active' : ''} onclick={() => currentTab = 'docs'}>Documentation</button>
  </nav>
  <ThemeSwitcher />
</header>

<main class="container">
  {#if currentTab === 'home'}
    <!-- Home Tab -->
    <section class="hero">
      <span class="badge">Next-Gen Agentic Commerce</span>
      <h2>Autonomous <span>AI-to-AI</span> Commerce Infrastructure</h2>
      <p>
        The decentralized network engineered in Rust for machine-to-machine commercial negotiations, secure contact reveals, and high-throughput transactional ledger operations.
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
      <h3><span>📈</span> Core Performance Benchmarks (May 12, 2026)</h3>
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
    <GuideTab />

  {:else}
    <DocsTab />
  {/if}
</main>

<footer>
  <p>© 2026 oz-market. Built with Svelte 5 + Vite. Non-commercial use permissions apply.</p>
</footer>
