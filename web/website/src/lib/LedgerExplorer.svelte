<script>
  import { sim, resetSeedLedger } from './simulator.svelte.js';

  let searchQuery = $state('');

  let filteredBlocks = $derived(
    sim.committedBlocks.filter(b =>
      b.item.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  let totalVolume = $derived(filteredBlocks.reduce((sum, b) => sum + b.price, 0));

  let ledgerContainer = $state();

  // Auto-scroll to top when a new block is added
  $effect(() => {
    filteredBlocks.length; // track length changes only
    if (ledgerContainer) {
      requestAnimationFrame(() => {
        ledgerContainer.scrollTop = 0;
      });
    }
  });
</script>

<style>
  .ledger-explorer {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .ledger-title {
    font-size: 0.78rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: 0.6rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .ledger-title .ledger-live-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-success);
    box-shadow: 0 0 6px var(--color-success);
    animation: pulse 2s infinite;
  }

  @keyframes pulse {
    0% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(0, 242, 254, 0.7); }
    70% { transform: scale(1); box-shadow: 0 0 0 10px rgba(0, 242, 254, 0); }
    100% { transform: scale(0.95); box-shadow: 0 0 0 0 rgba(0, 242, 254, 0); }
  }

  .ledger-search-container {
    position: relative;
    margin-bottom: 0.75rem;
  }

  .ledger-search-input {
    width: 100%;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--border-glow);
    border-radius: 8px;
    padding: 0.4rem 1.8rem 0.4rem 0.6rem;
    font-family: var(--font-sans);
    font-size: 0.75rem;
    color: var(--text-primary);
    outline: none;
    transition: border-color 0.2s ease, box-shadow 0.2s ease;
  }

  .ledger-search-input:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 8px var(--color-primary-glow);
  }

  .ledger-clear-btn {
    position: absolute;
    right: 0.5rem;
    top: 50%;
    transform: translateY(-50%);
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0;
    line-height: 1;
  }

  .ledger-clear-btn:hover {
    color: var(--text-primary);
  }

  .ledger-blocks {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 200px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--border-glow) transparent;
  }

  .ledger-block {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--border-glow);
    border-radius: 8px;
    padding: 0.6rem 0.75rem;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    animation: blockSlideIn 0.4s ease;
  }

  .ledger-block.new {
    border-color: var(--color-success);
    box-shadow: 0 0 10px rgba(var(--color-success), 0.15);
  }

  @keyframes blockSlideIn {
    from { opacity: 0; transform: translateY(-8px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .ledger-block-hash {
    color: var(--color-secondary);
    font-weight: 500;
    margin-bottom: 0.2rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ledger-block-meta {
    color: var(--text-muted);
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .ledger-block-price {
    color: var(--color-accent);
    font-weight: 700;
  }

  .ledger-count-badge {
    margin-left: auto;
    font-family: var(--font-mono);
    font-size: 0.68rem;
    font-weight: 600;
    color: var(--color-primary);
    background: var(--color-primary-glow);
    border: 1px solid var(--border-glow);
    border-radius: 20px;
    padding: 0.1rem 0.5rem;
    letter-spacing: 0.5px;
  }

  .ledger-volume {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 0.6rem;
    padding-top: 0.6rem;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    font-family: var(--font-mono);
    font-size: 0.72rem;
  }

  .ledger-volume-label {
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.8px;
  }

  .ledger-volume-value {
    color: var(--color-accent);
    font-weight: 700;
    font-size: 0.8rem;
  }

  .ledger-reset-btn {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--text-muted);
    font-family: var(--font-sans);
    font-size: 0.65rem;
    font-weight: 600;
    text-transform: uppercase;
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.2s, color 0.2s, border-color 0.2s;
  }

  .ledger-reset-btn:hover {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-primary);
    border-color: rgba(255, 255, 255, 0.3);
  }
</style>

<div class="ledger-explorer">
  <div class="ledger-title">
    <span class="ledger-live-dot"></span>
    Ledger Cache
    <span class="ledger-count-badge">{filteredBlocks.length} blocks</span>
  </div>
  <div class="ledger-search-container">
    <input
      type="text"
      placeholder="Filter by item..."
      bind:value={searchQuery}
      class="ledger-search-input"
    />
    {#if searchQuery}
      <button class="ledger-clear-btn" onclick={() => searchQuery = ''}>×</button>
    {/if}
  </div>
  <div class="ledger-blocks" bind:this={ledgerContainer}>
    {#each filteredBlocks as block}
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
  <!-- Total traded volume -->
  <div class="ledger-volume">
    <span class="ledger-volume-label">Total Volume</span>
    <span class="ledger-volume-value">${totalVolume.toLocaleString()}</span>
    <button class="ledger-reset-btn" onclick={() => { resetSeedLedger(); searchQuery = ''; }} title="Reset Ledger to Default">Reset</button>
  </div>
</div>
