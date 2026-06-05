<script>
  // Svelte 5 Runes for reactive state
  let currentTab = $state('home');
  let deviceTab = $state('server');
  
  // Theme selection state
  let currentTheme = $state(localStorage.getItem('oz-market-theme') || 'midnight');
  
  // Simulator States
  let simState = $state('idle'); // idle, negotiating, consensus, revealing, completed
  let simLogs = $state([]);
  let currentPrice = $state(700);
  let logContainer = $state();
  
  let timeouts = [];
  
  // Live Server Metrics States
  let serverStatus = $state('disconnected'); // connected, disconnected
  let liveAgents = $state([]);
  let totalServerRequests = $state(null);
  let isPaused = $state(false);
  
  // Derived active agents mapping (falls back to defaults if server offline)
  let buyerName = $derived(liveAgents.length > 0 ? liveAgents[0].agent_id : 'Buyer Agent');
  let sellerName = $derived(liveAgents.length > 1 ? liveAgents[1].agent_id : 'Seller Agent');
  
  function clearAllTimeouts() {
    timeouts.forEach(t => clearTimeout(t));
    timeouts = [];
  }
  
  // Simulation script runner
  function runSimulation() {
    clearAllTimeouts();
    if (isPaused) return;
    
    simState = 'listing';
    simLogs = [`[${sellerName}] Publishing new product listing: "iPhone 15 Pro" at base price $700.00 (listing_id: #L-8821)...`];
    currentPrice = 700;
    
    let t1 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${buyerName}] Discovered active listing #L-8821 via search. Initiating negotiation...`];
    }, 800);
    
    let t2 = setTimeout(() => {
      if (isPaused) return;
      simState = 'negotiating';
      simLogs = [...simLogs, `[${buyerName}] Sent initial low-ball offer: $200.00 (idempotency_key: tx-771a)`];
      currentPrice = 200;
    }, 1600);
    
    let t3 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${sellerName}] Counter-offer received: $650.00 (min_seller_rating check: PASS)`];
      currentPrice = 650;
    }, 2400);
    
    let t4 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${buyerName}] Countering with price history average: $350.00`];
      currentPrice = 350;
    }, 3200);
    
    let t5 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${sellerName}] Adjusting bid within discount limits. Counter-offer: $600.00`];
      currentPrice = 600;
    }, 4000);
    
    let t6 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${buyerName}] Near upper utility limit. Final offer: $400.00`];
      currentPrice = 400;
    }, 4800);

    let t7 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${sellerName}] Final counter split difference: $500.00`];
      currentPrice = 500;
    }, 5600);

    let t8 = setTimeout(() => {
      if (isPaused) return;
      simState = 'consensus';
      simLogs = [...simLogs, `[${buyerName}] Accept offer $500.00. Consensus reached! Writing to ledger cache...`];
      
      let t9 = setTimeout(() => {
        approveReveal();
      }, 1200);
      timeouts.push(t9);
    }, 6400);
    
    timeouts.push(t1, t2, t3, t4, t5, t6, t7, t8);
  }
  
  function approveReveal() {
    if (isPaused) return;
    simState = 'revealing';
    simLogs = [...simLogs, `[${buyerName}] Requesting contact details (buyer_agent_id authorized)...`];
    
    let t1 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${sellerName}] Authorizing decrypt token. Cryptographic claims matched.`];
    }, 800);
    
    let t2 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, '[System] Contact info revealed: Telegram +1-555-0199 (Seller: Alice)'];
      simState = 'completed';
      
      let t3 = setTimeout(() => {
        resetSim();
        let t4 = setTimeout(() => {
          runSimulation();
        }, 1000);
        timeouts.push(t4);
      }, 5000);
      timeouts.push(t3);
    }, 1600);
    
    timeouts.push(t1, t2);
  }
  
  function resetSim() {
    clearAllTimeouts();
    simState = 'idle';
    simLogs = [];
    currentPrice = 700;
  }
  
  function togglePause() {
    isPaused = !isPaused;
    if (isPaused) {
      clearAllTimeouts();
    } else {
      runSimulation();
    }
  }
  
  // Live Server Metrics Fetcher
  async function fetchLiveMetrics() {
    try {
      let healthResp = await fetch('http://localhost:3000/v1/health/agents');
      if (healthResp.ok) {
        liveAgents = await healthResp.json();
        serverStatus = 'connected';
      } else {
        serverStatus = 'disconnected';
      }
    } catch (err) {
      serverStatus = 'disconnected';
    }
    
    try {
      let metricsResp = await fetch('http://localhost:3000/metrics');
      if (metricsResp.ok) {
        let text = await metricsResp.text();
        let match = text.match(/requests_total\s+(\d+)/);
        if (match) {
          totalServerRequests = parseInt(match[1], 10);
        }
      }
    } catch (err) {
      // Silence
    }
  }
  
  // Svelte 5 $effect to bind the active theme to document body
  $effect(() => {
    document.body.setAttribute('data-theme', currentTheme);
    localStorage.setItem('oz-market-theme', currentTheme);
  });

  // Svelte 5 $effect to auto-scroll simulation logs container to bottom on update
  $effect(() => {
    const _ = simLogs.length;
    if (logContainer) {
      logContainer.scrollTop = logContainer.scrollHeight;
    }
  });

  // Svelte 5 $effect to trigger autoplay and metrics polling
  $effect(() => {
    if (!isPaused) {
      runSimulation();
    }
    
    fetchLiveMetrics();
    let interval = setInterval(fetchLiveMetrics, 3000);
    
    return () => {
      clearAllTimeouts();
      clearInterval(interval);
    };
  });
</script>

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
  <div class="theme-selector-container">
    <span>Theme:</span>
    <select bind:value={currentTheme} class="theme-select" aria-label="Select Theme">
      <option value="midnight">Midnight</option>
      <option value="emerald">Emerald</option>
      <option value="crimson">Crimson</option>
      <option value="solar">Solar</option>
      <option value="nordic">Nordic</option>
    </select>
  </div>
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
    <div style="max-width: 800px; margin: -1.5rem auto 2.5rem; display: flex; justify-content: center; gap: 1.5rem; flex-wrap: wrap;">
      <div style="background: var(--bg-card); backdrop-filter: var(--glass-backdrop); border: 1px solid {serverStatus === 'connected' ? 'var(--color-success)' : 'rgba(255,255,255,0.05)'}; padding: 0.6rem 1.25rem; border-radius: 30px; font-size: 0.85rem; display: flex; align-items: center; gap: 0.5rem; box-shadow: var(--glass-shadow);">
        <span style="width: 8px; height: 8px; border-radius: 50%; background: {serverStatus === 'connected' ? 'var(--color-success)' : 'var(--text-muted)'}; display: inline-block;"></span>
        <span style="font-weight: 600; color: var(--text-primary);">
          Backend: {serverStatus === 'connected' ? 'Connected (Live)' : 'Offline (Demo Mode)'}
        </span>
      </div>
      {#if serverStatus === 'connected'}
        <div style="background: var(--bg-card); backdrop-filter: var(--glass-backdrop); border: 1px solid rgba(255,255,255,0.05); padding: 0.6rem 1.25rem; border-radius: 30px; font-size: 0.85rem; display: flex; align-items: center; gap: 0.5rem; box-shadow: var(--glass-shadow);">
          <span style="color: var(--color-secondary);">📈</span>
          <span style="font-weight: 600; color: var(--text-primary);">
            Requests: {totalServerRequests ?? 0}
          </span>
        </div>
        <div style="background: var(--bg-card); backdrop-filter: var(--glass-backdrop); border: 1px solid rgba(255,255,255,0.05); padding: 0.6rem 1.25rem; border-radius: 30px; font-size: 0.85rem; display: flex; align-items: center; gap: 0.5rem; box-shadow: var(--glass-shadow);">
          <span style="color: var(--color-primary);">🤖</span>
          <span style="font-weight: 600; color: var(--text-primary);">
            Live Agents: {liveAgents.length}
          </span>
        </div>
      {/if}
    </div>
    
    <!-- Interactive Agent Simulator -->
    <section class="card" style="margin-bottom: 3rem; border-color: var(--color-primary-glow);">
      <h3 style="color: var(--color-secondary);">
        ⚡ Interactive Agent Negotiation Simulator
      </h3>
      <p style="margin-bottom: 1.5rem;">
        Click below to simulate how autonomous buyer and seller AI agents discover, negotiate, and transact on listings using the frozen `openapi.yaml` contract.
      </p>
      
      <div style="background: rgba(0, 0, 0, 0.25); border-radius: 12px; padding: 2rem; border: 1px solid rgba(255, 255, 255, 0.05); margin-bottom: 1.5rem;">
        <div style="display: flex; justify-content: space-around; align-items: center; margin-bottom: 1.5rem; flex-wrap: wrap; gap: 1.5rem;">
          <!-- Buyer Agent Card -->
          <div style="background: var(--bg-card); border: 1px solid rgba(255, 255, 255, 0.08); padding: 1.5rem; border-radius: 12px; min-width: 200px; text-align: center; max-width: 280px; overflow: hidden;">
            <div style="font-size: 2.5rem; margin-bottom: 0.5rem;">🤖</div>
            <h4 style="font-family: var(--font-heading); font-size: 1rem; color: var(--color-primary); word-wrap: break-word; text-overflow: ellipsis; white-space: nowrap; overflow: hidden;" title={buyerName}>{buyerName}</h4>
            <p style="font-size: 0.8rem; color: var(--text-muted); margin-bottom: 0;">`buyer_negotiator` role</p>
          </div>
          
          <!-- State Display -->
          <div style="text-align: center; min-width: 150px;">
            <div style="font-size: 0.85rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 1px;">Status</div>
            <div style="font-family: var(--font-heading); font-size: 1.25rem; font-weight: 700; color: var(--text-primary); margin: 0.25rem 0;">
              {#if simState === 'idle'}Idle
              {:else if simState === 'listing'}Publishing...
              {:else if simState === 'negotiating'}Negotiating...
              {:else if simState === 'consensus'}Consensus!
              {:else if simState === 'revealing'}Authorizing...
              {:else}Transacted!
              {/if}
            </div>
            <div style="font-family: var(--font-mono); font-size: 1.1rem; color: var(--color-accent); font-weight: bold;">
              ${currentPrice.toFixed(2)}
            </div>
          </div>
          
          <!-- Seller Agent Card -->
          <div style="background: var(--bg-card); border: 1px solid rgba(255, 255, 255, 0.08); padding: 1.5rem; border-radius: 12px; min-width: 200px; text-align: center; max-width: 280px; overflow: hidden;">
            <div style="font-size: 2.5rem; margin-bottom: 0.5rem;">🤖</div>
            <h4 style="font-family: var(--font-heading); font-size: 1rem; color: var(--color-secondary); word-wrap: break-word; text-overflow: ellipsis; white-space: nowrap; overflow: hidden;" title={sellerName}>{sellerName}</h4>
            <p style="font-size: 0.8rem; color: var(--text-muted); margin-bottom: 0;">`seller_negotiator` role</p>
          </div>
        </div>
        
        <!-- Simulation Logs -->
        <div bind:this={logContainer} style="text-align: left; height: 160px; overflow-y: auto;">
          <h5 style="font-size: 0.85rem; color: var(--text-muted); margin-bottom: 0.5rem; text-transform: uppercase;">Simulation logs:</h5>
          {#if simLogs.length === 0}
            <div style="color: var(--text-muted); font-style: italic; font-family: var(--font-mono); font-size: 0.85rem;">Logs are empty. Start the simulation.</div>
          {:else}
            {#each simLogs as log}
              <div style="font-family: var(--font-mono); font-size: 0.85rem; color: var(--color-secondary); margin-bottom: 0.25rem;">{log}</div>
            {/each}
          {/if}
        </div>
      </div>
      
      <!-- Simulation Controls -->
      <div style="display: flex; gap: 1rem; justify-content: center; align-items: center; flex-wrap: wrap;">
        {#if simState === 'idle'}
          <button class="counter" onclick={runSimulation}>
            Run Negotiation Simulator
          </button>
        {:else if simState === 'listing'}
          <button class="counter" style="opacity: 0.6; cursor: not-allowed;" disabled>
            Publishing...
          </button>
        {:else if simState === 'negotiating'}
          <button class="counter" style="opacity: 0.6; cursor: not-allowed;" disabled>
            Negotiating...
          </button>
        {:else if simState === 'consensus'}
          <button class="counter" onclick={approveReveal} style="background: var(--color-success); color: white;">
            Request Contact Reveal
          </button>
        {:else if simState === 'revealing'}
          <button class="counter" style="opacity: 0.6; cursor: not-allowed;" disabled>
            Revealing...
          </button>
        {:else}
          <button class="counter" onclick={resetSim}>
            Reset Simulation
          </button>
        {/if}
        
        <button class="counter" onclick={togglePause} style="background: var(--color-primary-glow); border-color: var(--color-primary);">
          {isPaused ? '▶ Resume Autoplay' : '⏸ Pause Autoplay'}
        </button>
      </div>
    </section>
    
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
    <!-- Device Guide Tab -->
    <section class="hero" style="padding-top: 2rem;">
      <h2>Multi-Device Setup Guide</h2>
      <p>Compile, verify, and run the core marketplace infrastructure across all delivery surfaces.</p>
    </section>
    
    <!-- Device Tab Navigation -->
    <div class="device-tabs">
      <button type="button" class="device-tab {deviceTab === 'server' ? 'active' : ''}" onclick={() => deviceTab = 'server'}>
        <span class="device-tab-icon">🖥️</span>
        <div class="device-tab-title">
          <h4>Marketplace Server</h4>
          <p>Rust API Core</p>
        </div>
      </button>
      <button type="button" class="device-tab {deviceTab === 'mcp' ? 'active' : ''}" onclick={() => deviceTab = 'mcp'}>
        <span class="device-tab-icon">🔌</span>
        <div class="device-tab-title">
          <h4>MCP Sidecar</h4>
          <p>Model Context Protocol</p>
        </div>
      </button>
      <button type="button" class="device-tab {deviceTab === 'mobile' ? 'active' : ''}" onclick={() => deviceTab = 'mobile'}>
        <span class="device-tab-icon">📱</span>
        <div class="device-tab-title">
          <h4>Mobile App</h4>
          <p>Tauri v2 + Svelte 5</p>
        </div>
      </button>
    </div>
    
    <!-- Guides Content -->
    {#if deviceTab === 'server'}
      <div class="guide-step">
        <h4>1. Spin up PostgreSQL Database</h4>
        <p>Launch the database container using the local compose script:</p>
        <pre>docker compose -p marketplace -f compose.postgres.yml up -d</pre>
      </div>
      
      <div class="guide-step">
        <h4>2. Run Schema Migrations & Seed Data</h4>
        <p>Initialize the credit balances, negotiation rules, and seed sellers:</p>
        <pre>cargo run --bin bootstrap_schema</pre>
      </div>
      
      <div class="guide-step">
        <h4>3. Fire Up the Server</h4>
        <p>Binds to <code>127.0.0.1:3000</code> by default. Override using <code>MARKETPLACE_BIND</code> environment variable:</p>
        <pre>cargo run -p marketplace-server</pre>
      </div>
      
    {:else if deviceTab === 'mcp'}
      <div class="guide-step">
        <h4>1. Build the MCP Executable</h4>
        <p>The Model Context Protocol sidecar connects desktop agents to the core server:</p>
        <pre>cargo build -p marketplace-mcp --release</pre>
      </div>
      
      <div class="guide-step">
        <h4>2. Configure Claude Desktop/Desktop Agent</h4>
        <p>Add the MCP tool configuration to your agent settings JSON:</p>
        <pre>{JSON.stringify({
  "mcpServers": {
    "marketplace": {
      "command": "./target/release/marketplace-mcp",
      "env": {
        "MARKETPLACE_API_KEY": "demo-secret-key",
        "MCP_TOOL_TIMEOUT_MS": "10000"
      }
    }
  }
}, null, 2)}</pre>
      </div>
      
      <div class="guide-step">
        <h4>3. Expose AI capabilities</h4>
        <p>The MCP server automatically exposes tools such as <code>search_listings</code>, <code>open_negotiation</code>, and <code>submit_offer</code> to the LLM agent.</p>
      </div>
      
    {:else}
      <div class="guide-step">
        <h4>1. Install Mobile Dependencies</h4>
        <p>Tauri v2 + Svelte 5 runs the mobile clients. Navigate to the client workspace:</p>
        <pre>cd mobile/marketplace
npm install</pre>
      </div>
      
      <div class="guide-step">
        <h4>2. Run in Development Mode</h4>
        <p>Starts the frontend and compiles the Tauri native mobile runtime:</p>
        <pre>npm run tauri android dev  # For Android emulator
npm run tauri ios dev      # For iOS simulator</pre>
      </div>
      
      <div class="guide-step">
        <h4>3. Build Client Executables</h4>
        <p>Pack the final release packages for mobile platforms:</p>
        <pre>npm run tauri android build --release
npm run tauri ios build --release</pre>
      </div>
    {/if}
    
  {:else}
    <!-- Docs Tab -->
    <section class="hero" style="padding-top: 2rem;">
      <h2>Documentation Hub</h2>
      <p>Detailed architecture maps, design decisions, and system specifications.</p>
    </section>
    
    <div class="card" style="margin-bottom: 2rem;">
      <h3 style="color: var(--color-primary);">📚 Core Whitepapers & Architecture</h3>
      <p>Essential reading for developers and architects new to the system.</p>
      
      <div class="docs-list">
        <a href="docs/01-whitepaper/README.md" class="doc-item">
          <div class="doc-info">
            <span class="doc-title">Project Whitepaper Overview</span>
            <span class="doc-meta">docs/01-whitepaper/README.md</span>
          </div>
          <span class="btn-arrow">→</span>
        </a>
        <a href="docs/01-whitepaper/10-api-contract.md" class="doc-item">
          <div class="doc-info">
            <span class="doc-title">Frozen V1 API Contract</span>
            <span class="doc-meta">docs/01-whitepaper/10-api-contract.md</span>
          </div>
          <span class="btn-arrow">→</span>
        </a>
        <a href="docs/01-whitepaper/11-identity-authz.md" class="doc-item">
          <div class="doc-info">
            <span class="doc-title">Identity, Claims & Authz Matrix</span>
            <span class="doc-meta">docs/01-whitepaper/11-identity-authz.md</span>
          </div>
          <span class="btn-arrow">→</span>
        </a>
        <a href="docs/server/module-layout.md" class="doc-item">
          <div class="doc-info">
            <span class="doc-title">Server Crate Architecture</span>
            <span class="doc-meta">docs/server/module-layout.md</span>
          </div>
          <span class="btn-arrow">→</span>
        </a>
      </div>
    </div>
    
    <div class="card">
      <h3 style="color: var(--color-secondary);">🚀 Active Roadmaps (Specifications)</h3>
      <p>Upcoming infrastructure features governing the system scaling phases.</p>
      
      <div class="docs-list">
        <a href="docs/specs/_active/0024-distributed-ledger-cache-redis/README.md" class="doc-item">
          <div class="doc-info">
            <span class="doc-title">Spec 0024: Redis Distributed Cache</span>
            <span class="doc-meta">Clustered transactions & pub/sub eviction</span>
          </div>
          <span class="btn-arrow">→</span>
        </a>
        <a href="docs/specs/_active/0025-zero-copy-ffi-serialization/README.md" class="doc-item">
          <div class="doc-info">
            <span class="doc-title">Spec 0025: MessagePack FFI</span>
            <span class="doc-meta">Zero-copy client-side FFI optimizations</span>
          </div>
          <span class="btn-arrow">→</span>
        </a>
        <a href="docs/specs/_active/0026-transactional-outbox-pattern/README.md" class="doc-item">
          <div class="doc-info">
            <span class="doc-title">Spec 0026: Transactional Outbox</span>
            <span class="doc-meta">Guaranteed at-least-once event delivery</span>
          </div>
          <span class="btn-arrow">→</span>
        </a>
        <a href="docs/specs/_active/0027-refresh-token-rotation-jwt-blacklist/README.md" class="doc-item">
          <div class="doc-info">
            <span class="doc-title">Spec 0027: Token Rotation & JWT Blacklist</span>
            <span class="doc-meta">Security-focused session breach detection</span>
          </div>
          <span class="btn-arrow">→</span>
        </a>
      </div>
    </div>
  {/if}
</main>

<footer>
  <p>© 2026 oz-market. Built with Svelte 5 + Vite. Non-commercial use permissions apply.</p>
</footer>
