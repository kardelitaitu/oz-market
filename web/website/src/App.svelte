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
  let ledgerContainer = $state();

  // Live Ledger Explorer — seed historical blocks
  function randHash() {
    return '0x' + Math.floor(Math.random() * 0xffffffffffff).toString(16).padStart(12, '0');
  }
  let committedBlocks = $state([
    { hash: '0x3f8a21c94d07', price: 480,  item: 'Samsung Galaxy S24 Ultra', ts: '17:58:02', isNew: false },
    { hash: '0xa1b09e3f72cc', price: 320,  item: 'iPad Pro 11" M4',          ts: '17:41:35', isNew: false },
    { hash: '0x7cd45f183a92', price: 650,  item: 'MacBook Air M3',           ts: '17:22:19', isNew: false },
    { hash: '0xb2e70d9c15f1', price: 210,  item: 'Sony WH-1000XM5',          ts: '17:05:44', isNew: false },
    { hash: '0xf9a3c841de02', price: 890,  item: 'DJI Mini 4 Pro',           ts: '16:48:11', isNew: false },
    { hash: '0x0d72fe5b8a63', price: 145,  item: 'Apple AirPods Pro 2',      ts: '16:30:57', isNew: false },
    { hash: '0x5c1b947e2f80', price: 1100, item: 'ASUS ROG Zephyrus G14',    ts: '16:12:33', isNew: false },
    { hash: '0xe8d04c3791ab', price: 390,  item: 'Google Pixel 8 Pro',       ts: '15:55:08', isNew: false },
    { hash: '0x29fc8b60a347', price: 275,  item: 'Nintendo Switch OLED',     ts: '15:37:44', isNew: false },
    { hash: '0xc6a51e082d94', price: 560,  item: 'Samsung 49" Odyssey G9',   ts: '15:19:20', isNew: false },
    { hash: '0x84b3d7f91c50', price: 720,  item: 'iPhone 15 Pro Max',        ts: '15:02:55', isNew: false },
    { hash: '0x1a9e5042bc76', price: 430,  item: 'Fujifilm X-T5 Body',       ts: '14:44:31', isNew: false },
    { hash: '0x6d27cf3e54b8', price: 185,  item: 'Meta Quest 3 (128GB)',      ts: '14:27:06', isNew: false },
    { hash: '0xd50e8f1a7263', price: 310,  item: 'Garmin Fenix 7S Pro',      ts: '14:09:42', isNew: false },
    { hash: '0x92bc4d6f3e01', price: 95,   item: 'Anker 737 Power Bank',     ts: '13:52:18', isNew: false },
  ]);

  
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

  // Returns a CSS color variable string based on who is speaking in a log line
  function logColor(log) {
    if (log.startsWith('[System]'))  return 'var(--color-success)';
    if (log.startsWith(`[${sellerName}]`) || log.startsWith('[Seller Agent]')) return 'var(--color-secondary)';
    return 'var(--color-primary)';
  }

  // Splits a log line into a [Tag] prefix and the rest of the message
  function logParts(log) {
    const m = log.match(/^(\[[^\]]+\])\s*(.*)$/);
    return m ? { tag: m[1], msg: m[2] } : { tag: '', msg: log };
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
    }, 1800);
    
    let t2 = setTimeout(() => {
      if (isPaused) return;
      simState = 'negotiating';
      simLogs = [...simLogs, `[${buyerName}] Sent initial low-ball offer: $200.00 (idempotency_key: tx-771a)`];
      currentPrice = 200;
    }, 3600);
    
    let t3 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${sellerName}] Counter-offer received: $650.00 (min_seller_rating check: PASS)`];
      currentPrice = 650;
    }, 5400);
    
    let t4 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${buyerName}] Countering with price history average: $350.00`];
      currentPrice = 350;
    }, 7200);
    
    let t5 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${sellerName}] Adjusting bid within discount limits. Counter-offer: $600.00`];
      currentPrice = 600;
    }, 9000);
    
    let t6 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${buyerName}] Near upper utility limit. Final offer: $400.00`];
      currentPrice = 400;
    }, 10800);

    let t7 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${sellerName}] Final counter split difference: $500.00`];
      currentPrice = 500;
    }, 12600);

    let t8 = setTimeout(() => {
      if (isPaused) return;
      simState = 'consensus';
      simLogs = [...simLogs, `[${buyerName}] Accept offer $500.00. Consensus reached! Writing to ledger cache...`];

      // Commit a new ledger block
      const newBlock = {
        hash: randHash(),
        price: 500,
        item: 'iPhone 15 Pro',
        ts: new Date().toLocaleTimeString('en-US', { hour12: false }),
        isNew: true,
      };
      committedBlocks = [newBlock, ...committedBlocks];
      if (ledgerContainer) ledgerContainer.scrollTop = 0;
      setTimeout(() => {
        committedBlocks = committedBlocks.map((b, i) => i === 0 ? { ...b, isNew: false } : b);
      }, 600);

      let t9 = setTimeout(() => {
        approveReveal();
      }, 2000);
      timeouts.push(t9);
    }, 14400);
    
    timeouts.push(t1, t2, t3, t4, t5, t6, t7, t8);
  }
  
  function approveReveal() {
    if (isPaused) return;
    simState = 'revealing';
    simLogs = [...simLogs, `[${buyerName}] Requesting contact details (buyer_agent_id authorized)...`];
    
    let t1 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, `[${sellerName}] Authorizing decrypt token. Cryptographic claims matched.`];
    }, 1800);
    
    let t2 = setTimeout(() => {
      if (isPaused) return;
      simLogs = [...simLogs, '[System] Contact info revealed: Telegram +1-555-0199 (Seller: Alice)'];
      simState = 'completed';
      
      let t3 = setTimeout(() => {
        resetSim();
        let t4 = setTimeout(() => {
          runSimulation();
        }, 2000);
        timeouts.push(t4);
      }, 8000);
      timeouts.push(t3);
    }, 3600);
    
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
  <div class="theme-swatches" role="group" aria-label="Select Theme">
    <span class="theme-swatches-label">Theme</span>
    {#each [
      { id: 'midnight', label: 'Midnight' },
      { id: 'emerald',  label: 'Emerald'  },
      { id: 'crimson',  label: 'Crimson'  },
      { id: 'solar',    label: 'Solar'    },
      { id: 'nordic',   label: 'Nordic'   },
    ] as th}
      <button
        class="theme-swatch swatch-{th.id} {currentTheme === th.id ? 'active' : ''}"
        aria-label="{th.label} theme"
        aria-pressed={currentTheme === th.id}
        title={th.label}
        onclick={() => currentTheme = th.id}
      ></button>
    {/each}
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

        <!-- Agent Cards Row -->
        <div style="display: flex; justify-content: space-around; align-items: center; margin-bottom: 1.5rem; flex-wrap: wrap; gap: 1.5rem;">

          <!-- Buyer Agent Card -->
          <div style="background: var(--bg-card); border: 1px solid {simState === 'negotiating' || simState === 'revealing' ? 'var(--color-primary)' : 'rgba(255,255,255,0.08)'}; padding: 1.5rem; border-radius: 12px; min-width: 160px; text-align: center; max-width: 220px; overflow: hidden; transition: border-color 0.4s ease; box-shadow: {simState === 'negotiating' || simState === 'revealing' ? '0 0 14px var(--color-primary-glow)' : 'none'};">
            <div style="font-size: 2.5rem; margin-bottom: 0.5rem;">🤖</div>
            <h4 style="font-family: var(--font-heading); font-size: 1rem; color: var(--color-primary); word-wrap: break-word; text-overflow: ellipsis; white-space: nowrap; overflow: hidden;" title={buyerName}>{buyerName}</h4>
            <p style="font-size: 0.8rem; color: var(--text-muted); margin-bottom: 0;">`buyer_negotiator` role</p>
          </div>

          <!-- State Display + SVG Flow -->
          <div style="text-align: center; min-width: 160px; flex: 1; max-width: 280px;">

            <!-- SVG Architecture Flow Diagram — pure SVG geometry, no emoji -->
            <svg class="flow-diagram" viewBox="0 0 320 80" xmlns="http://www.w3.org/2000/svg" aria-label="Agent negotiation flow diagram">
              <defs>
                <!-- Glow filter for active nodes -->
                <filter id="node-glow" x="-50%" y="-50%" width="200%" height="200%">
                  <feGaussianBlur stdDeviation="3" result="blur"/>
                  <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
                </filter>
              </defs>

              <!-- ─── Connector lines ─── -->
              <line
                class="flow-path {simState !== 'idle' ? 'active' : ''}"
                x1="68" y1="40" x2="134" y2="40"
              />
              <line
                class="flow-path {simState === 'listing' || simState === 'negotiating' || simState === 'consensus' || simState === 'revealing' || simState === 'completed' ? 'active' : ''}"
                x1="186" y1="40" x2="256" y2="40"
              />

              <!-- ─── Signal dots: inline path avoids mpath ID lookup failures ─── -->

              <!-- Buyer→Server forward dot (active when sim is running) -->
              {#if simState !== 'idle'}
                <circle r="3.5" fill="var(--color-secondary)" opacity="0.9">
                  <animateMotion
                    dur="1.1s"
                    repeatCount="indefinite"
                    calcMode="linear"
                    path="M 68,40 L 134,40"
                  />
                </circle>
              {/if}

              <!-- Server→Buyer return dot (only during negotiation for counter-offer feel) -->
              {#if simState === 'negotiating'}
                <circle r="2.8" fill="var(--color-accent)" opacity="0.75">
                  <animateMotion
                    dur="1.1s"
                    repeatCount="indefinite"
                    calcMode="linear"
                    path="M 134,40 L 68,40"
                    begin="0.55s"
                  />
                </circle>
              {/if}

              <!-- Server→Seller forward dot -->
              {#if simState === 'listing' || simState === 'negotiating' || simState === 'consensus' || simState === 'revealing' || simState === 'completed'}
                <circle r="3.5" fill="var(--color-secondary)" opacity="0.9">
                  <animateMotion
                    dur="1.1s"
                    repeatCount="indefinite"
                    calcMode="linear"
                    path="M 186,40 L 256,40"
                  />
                </circle>
              {/if}

              <!-- Seller→Server return dot (during negotiation) -->
              {#if simState === 'negotiating'}
                <circle r="2.8" fill="var(--color-accent)" opacity="0.75">
                  <animateMotion
                    dur="1.1s"
                    repeatCount="indefinite"
                    calcMode="linear"
                    path="M 256,40 L 186,40"
                    begin="0.55s"
                  />
                </circle>
              {/if}

              <!-- ─── BUYER node (cx=40) ─── -->
              <g filter="url(#node-glow)">
                <circle
                  class="flow-node-circle"
                  cx="40" cy="40" r="24"
                  style="stroke: {simState === 'negotiating' || simState === 'revealing' ? 'var(--color-secondary)' : 'var(--color-primary)'}; stroke-width: {simState === 'negotiating' || simState === 'revealing' ? '2' : '1.5'};"
                />
                <!-- Head -->
                <circle cx="40" cy="31" r="5" fill="var(--color-primary)" opacity="0.9"/>
                <!-- Body -->
                <path d="M 33 43 Q 40 38 47 43 L 47 52 L 33 52 Z" fill="var(--color-primary)" opacity="0.7"/>
              </g>
              <text class="flow-node-label" x="40" y="73">BUYER</text>

              <!-- ─── SERVER node (cx=160) ─── -->
              <g filter="url(#node-glow)">
                <circle
                  class="flow-node-circle"
                  cx="160" cy="40" r="26"
                  style="stroke: {simState !== 'idle' ? 'var(--color-secondary)' : 'var(--color-primary)'}; stroke-width: {simState !== 'idle' ? '2' : '1.5'};"
                />
                <!-- Gear ring + hub -->
                <circle cx="160" cy="40" r="9" fill="none" stroke="var(--color-secondary)" stroke-width="1.5" opacity="0.8"/>
                <circle cx="160" cy="40" r="4" fill="var(--color-secondary)" opacity="0.8"/>
                <!-- 8 gear teeth -->
                {#each [0,45,90,135,180,225,270,315] as deg}
                  <rect
                    x="158.5" y="27"
                    width="3" height="5"
                    fill="var(--color-secondary)"
                    opacity="0.75"
                    transform="rotate({deg} 160 40)"
                  />
                {/each}
              </g>
              <text class="flow-node-label" x="160" y="75">SERVER</text>

              <!-- ─── SELLER node (cx=280) ─── -->
              <g filter="url(#node-glow)">
                <circle
                  class="flow-node-circle"
                  cx="280" cy="40" r="24"
                  style="stroke: {simState === 'listing' || simState === 'negotiating' || simState === 'consensus' ? 'var(--color-secondary)' : 'var(--color-primary)'}; stroke-width: {simState === 'listing' || simState === 'negotiating' || simState === 'consensus' ? '2' : '1.5'};"
                />
                <!-- Head -->
                <circle cx="280" cy="31" r="5" fill="var(--color-secondary)" opacity="0.9"/>
                <!-- Body -->
                <path d="M 273 43 Q 280 38 287 43 L 287 52 L 273 52 Z" fill="var(--color-secondary)" opacity="0.7"/>
              </g>
              <text class="flow-node-label" x="280" y="73">SELLER</text>

              <!-- ─── Consensus ring: rotating dashed ring around server ─── -->
              {#if simState === 'consensus' || simState === 'revealing' || simState === 'completed'}
                <circle cx="160" cy="40" r="32" fill="none"
                  stroke="var(--color-success)" stroke-width="1.5"
                  stroke-dasharray="5 3" opacity="0.6"
                >
                  <animateTransform attributeName="transform" type="rotate"
                    from="0 160 40" to="360 160 40" dur="3s" repeatCount="indefinite"/>
                </circle>
              {/if}
            </svg>

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
          <div style="background: var(--bg-card); border: 1px solid {simState === 'listing' || simState === 'negotiating' || simState === 'consensus' ? 'var(--color-secondary)' : 'rgba(255,255,255,0.08)'}; padding: 1.5rem; border-radius: 12px; min-width: 160px; text-align: center; max-width: 220px; overflow: hidden; transition: border-color 0.4s ease; box-shadow: {simState === 'listing' || simState === 'negotiating' || simState === 'consensus' ? '0 0 14px var(--color-secondary-glow)' : 'none'};">
            <div style="font-size: 2.5rem; margin-bottom: 0.5rem;">🤖</div>
            <h4 style="font-family: var(--font-heading); font-size: 1rem; color: var(--color-secondary); word-wrap: break-word; text-overflow: ellipsis; white-space: nowrap; overflow: hidden;" title={sellerName}>{sellerName}</h4>
            <p style="font-size: 0.8rem; color: var(--text-muted); margin-bottom: 0;">`seller_negotiator` role</p>
          </div>
        </div>

        <!-- Sim Split: Logs + Ledger Explorer -->
        <div class="sim-split">

          <!-- Column 1: Simulation Logs -->
          <div>
            <div bind:this={logContainer} style="text-align: left; height: 175px; overflow-y: auto;">
              <h5 style="font-size: 0.85rem; color: var(--text-muted); margin-bottom: 0.5rem; text-transform: uppercase;">Simulation logs:</h5>
              {#if simLogs.length === 0}
                <div style="color: var(--text-muted); font-style: italic; font-family: var(--font-mono); font-size: 0.85rem;">Logs are empty. Start the simulation.</div>
              {:else}
                {#each simLogs as log}
                  {@const p = logParts(log)}
                  <div style="font-family: var(--font-mono); font-size: 0.82rem; margin-bottom: 0.35rem; line-height: 1.45;">
                    <span style="color: {logColor(log)}; font-weight: 700; opacity: 0.95;">{p.tag}</span>
                    {#if p.tag}<span style="color: var(--text-muted); margin: 0 0.15rem;"> </span>{/if}
                    <span style="color: var(--text-secondary);">{p.msg}</span>
                  </div>
                {/each}
              {/if}
            </div>
          </div>

          <!-- Column 2: Live Ledger Block Explorer -->
          <div class="ledger-explorer">
            <div class="ledger-title">
              <span class="ledger-live-dot"></span>
              Ledger Cache
            </div>
            <div class="ledger-blocks" bind:this={ledgerContainer}>
              {#each committedBlocks as block}
                <div class="ledger-block {block.isNew ? 'new' : ''}">
                  <div class="ledger-block-hash">{block.hash}</div>
                  <div class="ledger-block-meta">
                    <span>{block.item}</span>
                    <span class="ledger-block-price">${block.price}.00</span>
                  </div>
                  <div class="ledger-block-meta" style="margin-top: 0.2rem;">
                    <span>⏱ {block.ts}</span>
                    <span style="color: var(--color-success);">✓ committed</span>
                  </div>
                </div>
              {/each}
            </div>
          </div>

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
