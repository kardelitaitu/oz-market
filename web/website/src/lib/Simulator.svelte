<script>
  import FlowDiagram from './FlowDiagram.svelte';
  import AgentCard from './AgentCard.svelte';
  import LedgerExplorer from './LedgerExplorer.svelte';
  import MetricsPanel from './MetricsPanel.svelte';
  import {
    sim,
    runSimulation, approveReveal, resetSim, togglePause,
  } from './simulator.svelte.js';

  let buyerName = $derived(sim.liveAgents.length > 0 ? sim.liveAgents[0].agent_id : 'Buyer Agent');
  let sellerName = $derived(sim.liveAgents.length > 1 ? sim.liveAgents[1].agent_id : 'Seller Agent');

  let logContainer = $state();

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

  // Auto-scroll simulation logs container to bottom on update
  $effect(() => {
    const _ = sim.logs.length;
    if (logContainer) {
      logContainer.scrollTop = logContainer.scrollHeight;
    }
  });
</script>

<style>
  .counter {
    font-family: var(--font-sans);
    font-weight: 600;
    font-size: 0.85rem;
    padding: 0.6rem 1.25rem;
    border-radius: 20px;
    cursor: pointer;
    border: 1px solid var(--color-primary);
    background: var(--color-primary);
    color: var(--text-primary);
    transition: background-color 0.2s ease, box-shadow 0.2s ease;
  }

  .counter:hover:not(:disabled) {
    box-shadow: 0 4px 14px var(--color-primary-glow);
  }

  .counter:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .sim-split {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.5rem;
    align-items: start;
  }

  @media (max-width: 700px) {
    .sim-split {
      grid-template-columns: 1fr;
    }
  }
</style>

<section class="card" style="margin-bottom: 3rem; border-color: var(--color-primary-glow);">
  <h2 style="color: var(--color-secondary); font-size: 1.35rem; font-family: var(--font-heading); font-weight: 700; margin-bottom: 1rem;">
    ⚡ Interactive Agent Negotiation Simulator
  </h2>
  <p style="margin-bottom: 1.5rem;">
    Click below to simulate how autonomous buyer and seller AI agents discover, negotiate, and transact on listings using the frozen `openapi.yaml` contract.
  </p>

  <div style="background: rgba(0, 0, 0, 0.25); border-radius: 12px; padding: 2rem; border: 1px solid rgba(255, 255, 255, 0.05); margin-bottom: 1.5rem;">

    <!-- Agent Cards Row -->
    <div style="display: flex; justify-content: space-around; align-items: center; margin-bottom: 1.5rem; flex-wrap: wrap; gap: 1.5rem;">

      <!-- Buyer Agent Card -->
      <AgentCard
        name={buyerName}
        role="buyer"
        isActive={sim.state === 'negotiating' || sim.state === 'revealing'}
        title="`buyer_negotiator` role"
      />

      <!-- State Display + SVG Flow -->
      <div style="text-align: center; min-width: 160px; flex: 1; max-width: 280px;">
        <FlowDiagram simState={sim.state} />

        <div style="font-size: 0.85rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 1px;">Status</div>

        <div style="font-family: var(--font-heading); font-size: 1.25rem; font-weight: 700; color: var(--text-primary); margin: 0.25rem 0;">
          {#if sim.state === 'idle'}Idle
          {:else if sim.state === 'listing'}Publishing...
          {:else if sim.state === 'negotiating'}Negotiating...
          {:else if sim.state === 'consensus'}Consensus!
          {:else if sim.state === 'revealing'}Authorizing...
          {:else}Transacted!
          {/if}
        </div>
        <div style="font-family: var(--font-mono); font-size: 1.1rem; color: var(--color-accent); font-weight: bold;">
          ${sim.currentPrice.toFixed(2)}
        </div>
      </div>

      <!-- Seller Agent Card -->
      <AgentCard
        name={sellerName}
        role="seller"
        isActive={sim.state === 'listing' || sim.state === 'negotiating' || sim.state === 'consensus'}
        title="`seller_negotiator` role"
      />
    </div>

    <!-- Sim Split: Logs + Ledger Explorer -->
    <div class="sim-split">

      <!-- Column 1: Simulation Logs -->
      <div>
        <div bind:this={logContainer} style="text-align: left; height: 175px; overflow-y: auto;" role="log" aria-live="polite" aria-atomic="false" aria-relevant="additions" tabindex="0">
          <h4 style="font-size: 0.85rem; color: var(--text-muted); margin-bottom: 0.5rem; text-transform: uppercase;">Simulation logs:</h4>
          {#if sim.logs.length === 0}
            <div style="color: var(--text-muted); font-style: italic; font-family: var(--font-mono); font-size: 0.85rem;">Logs are empty. Start the simulation.</div>
          {:else}
            {#each sim.logs as log}
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
      <LedgerExplorer />

    </div>
  </div>

  <!-- Simulation Controls -->
  <div style="display: flex; gap: 1rem; justify-content: center; align-items: center; flex-wrap: wrap;">
    {#if sim.state === 'idle'}
      <button class="counter" onclick={runSimulation}>
        Run Negotiation Simulator
      </button>
    {:else if sim.state === 'listing'}
      <button class="counter" style="opacity: 0.6; cursor: not-allowed;" disabled>
        Publishing...
      </button>
    {:else if sim.state === 'negotiating'}
      <button class="counter" style="opacity: 0.6; cursor: not-allowed;" disabled>
        Negotiating...
      </button>
    {:else if sim.state === 'consensus'}
      <button class="counter" onclick={approveReveal} style="background: var(--color-success); color: white;">
        Request Contact Reveal
      </button>
    {:else if sim.state === 'revealing'}
      <button class="counter" style="opacity: 0.6; cursor: not-allowed;" disabled>
        Revealing...
      </button>
    {:else}
      <button class="counter" onclick={resetSim}>
        Reset Simulation
      </button>
    {/if}

    <button class="counter" onclick={togglePause} style="background: var(--color-primary-glow); border-color: var(--color-primary);">
      {sim.isPaused ? '▶ Resume Autoplay' : '⏸ Pause Autoplay'}
    </button>
  </div>

  <MetricsPanel />
</section>
