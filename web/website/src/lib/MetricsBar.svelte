<script>
  import { sim } from './simulator.svelte.js';

  let statusLabel = $derived(
    sim.serverStatus === 'connected' ? 'Connected (Live)' :
    sim.reconnectAttempts > 0 ? `Reconnecting (attempt ${sim.reconnectAttempts})...` :
    'Offline (Demo Mode)'
  );
  let statusColor = $derived(
    sim.serverStatus === 'connected' ? 'var(--color-success)' :
    sim.reconnectAttempts > 0 ? 'var(--color-accent)' :
    'var(--text-muted)'
  );
  let borderColor = $derived(
    sim.serverStatus === 'connected' ? 'var(--color-success)' :
    sim.reconnectAttempts > 0 ? 'var(--color-accent)' :
    'rgba(255,255,255,0.05)'
  );
</script>

<style>
  .metrics-bar-row {
    display: flex;
    justify-content: center;
    gap: 1.5rem;
    flex-wrap: wrap;
    max-width: 800px;
    margin: -1.5rem auto 2.5rem;
  }

  .pill {
    background: var(--bg-card);
    backdrop-filter: blur(16px);
    border-radius: 30px;
    padding: 0.6rem 1.25rem;
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    box-shadow: 0 8px 32px 0 rgba(0, 0, 0, 0.37);
  }

  .pill-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
  }

  .pill span:last-child {
    font-weight: 600;
    color: var(--text-primary);
  }
</style>

<div class="metrics-bar-row">
  <div class="pill" style="border: 1px solid {borderColor};">
    <span class="pill-dot" style="background: {statusColor};"></span>
    <span>Backend: {statusLabel}</span>
  </div>
  {#if sim.serverStatus === 'connected'}
    <div class="pill" style="border: 1px solid rgba(255,255,255,0.05);">
      <span style="color: var(--color-secondary);">📈</span>
      <span>Requests: {sim.totalServerRequests ?? 0}</span>
    </div>
    <div class="pill" style="border: 1px solid rgba(255,255,255,0.05);">
      <span style="color: var(--color-primary);">🤖</span>
      <span>Live Agents: {sim.liveAgents.length}</span>
    </div>
  {/if}
</div>
