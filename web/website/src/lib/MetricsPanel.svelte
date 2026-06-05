<script>
  import { sim, catalog } from './simulator.svelte.js';

  // Calculate Average Discount reactive state
  let avgDiscount = $derived.by(() => {
    let totalDiscount = 0;
    let count = 0;
    for (let block of sim.committedBlocks) {
      const itemInfo = catalog.find(c => c.name === block.item);
      if (itemInfo) {
        const discount = ((itemInfo.basePrice - block.price) / itemInfo.basePrice) * 100;
        totalDiscount += discount;
        count++;
      }
    }
    return count > 0 ? (totalDiscount / count) : 26.5; // fallback baseline
  });

  // Calculate Success Rate reactive state
  let successRate = $derived.by(() => {
    const total = sim.successCount + sim.failedCount;
    return total > 0 ? (sim.successCount / total) * 100 : 92.5; // fallback baseline
  });
</script>

<style>
  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 1rem;
    margin-top: 1.5rem;
    width: 100%;
  }

  .metric-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-glow);
    border-radius: 12px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    position: relative;
    transition: transform 0.2s ease, border-color 0.2s ease;
  }

  .metric-card:hover {
    transform: translateY(-2px);
    border-color: var(--color-primary);
  }

  .metric-label {
    font-size: 0.68rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 0.5rem;
  }

  .metric-value {
    font-family: var(--font-heading);
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 0.4rem;
    line-height: 1.1;
  }

  .metric-bar-container {
    width: 100%;
    height: 4px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 2px;
    overflow: hidden;
    margin-top: 0.2rem;
  }

  .metric-bar {
    height: 100%;
    border-radius: 2px;
    transition: width 0.6s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .metric-desc {
    font-size: 0.62rem;
    color: var(--text-muted);
    margin-top: 0.25rem;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.72rem;
    font-weight: 600;
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    position: relative;
  }

  .status-dot::after {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    border-radius: 50%;
    transform: scale(1);
    animation: beacon 2s infinite;
  }

  .status-dot.connected {
    background: var(--color-success);
  }

  .status-dot.connected::after {
    background: var(--color-success);
    box-shadow: 0 0 8px var(--color-success);
  }

  .status-dot.disconnected {
    background: var(--color-primary);
  }

  .status-dot.disconnected::after {
    background: var(--color-primary);
    box-shadow: 0 0 8px var(--color-primary);
  }

  @keyframes beacon {
    0% { transform: scale(1); opacity: 0.8; }
    100% { transform: scale(2.4); opacity: 0; }
  }
</style>

<div class="metrics-grid">
  <!-- Total Blocks -->
  <div class="metric-card">
    <span class="metric-label">Ledger Blocks</span>
    <span class="metric-value" style="color: var(--color-secondary);">{sim.committedBlocks.length}</span>
    <span class="metric-desc">Consensus records</span>
  </div>

  <!-- Success Rate -->
  <div class="metric-card">
    <span class="metric-label">Success Rate</span>
    <span class="metric-value" style="color: var(--color-success);">{successRate.toFixed(1)}%</span>
    <div class="metric-bar-container">
      <div class="metric-bar" style="width: {successRate}%; background: var(--color-success);"></div>
    </div>
    <span class="metric-desc">{sim.successCount} wins / {sim.failedCount} aborts</span>
  </div>

  <!-- Average Discount -->
  <div class="metric-card">
    <span class="metric-label">Avg Discount</span>
    <span class="metric-value" style="color: var(--color-accent);">{avgDiscount.toFixed(1)}%</span>
    <div class="metric-bar-container">
      <div class="metric-bar" style="width: {avgDiscount * 2}%; background: var(--color-accent);"></div>
    </div>
    <span class="metric-desc">Below base price</span>
  </div>

  <!-- Connection Status -->
  <div class="metric-card" style="justify-content: center;">
    <span class="metric-label">Stream Sync</span>
    <div class="status-indicator">
      {#if sim.serverStatus === 'connected'}
        <span class="status-dot connected"></span>
        <span style="color: var(--color-success);">SSE Connected</span>
      {:else}
        <span class="status-dot disconnected"></span>
        <span style="color: var(--text-secondary);">Offline (Autoplay)</span>
      {/if}
    </div>
    <span class="metric-desc" style="margin-top: 0.5rem;">
      {#if sim.totalServerRequests !== null}
        {sim.totalServerRequests.toLocaleString()} reqs logged
      {:else}
        Demo simulation mode
      {/if}
    </span>
  </div>
</div>
