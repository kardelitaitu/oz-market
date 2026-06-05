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

<div style="max-width: 800px; margin: -1.5rem auto 2.5rem; display: flex; justify-content: center; gap: 1.5rem; flex-wrap: wrap;">
  <div style="background: var(--bg-card); backdrop-filter: var(--glass-backdrop); border: 1px solid {borderColor}; padding: 0.6rem 1.25rem; border-radius: 30px; font-size: 0.85rem; display: flex; align-items: center; gap: 0.5rem; box-shadow: var(--glass-shadow);">
    <span style="width: 8px; height: 8px; border-radius: 50%; background: {statusColor}; display: inline-block;"></span>
    <span style="font-weight: 600; color: var(--text-primary);">
      Backend: {statusLabel}
    </span>
  </div>
  {#if sim.serverStatus === 'connected'}
    <div style="background: var(--bg-card); backdrop-filter: var(--glass-backdrop); border: 1px solid rgba(255,255,255,0.05); padding: 0.6rem 1.25rem; border-radius: 30px; font-size: 0.85rem; display: flex; align-items: center; gap: 0.5rem; box-shadow: var(--glass-shadow);">
      <span style="color: var(--color-secondary);">📈</span>
      <span style="font-weight: 600; color: var(--text-primary);">
        Requests: {sim.totalServerRequests ?? 0}
      </span>
    </div>
    <div style="background: var(--bg-card); backdrop-filter: var(--glass-backdrop); border: 1px solid rgba(255,255,255,0.05); padding: 0.6rem 1.25rem; border-radius: 30px; font-size: 0.85rem; display: flex; align-items: center; gap: 0.5rem; box-shadow: var(--glass-shadow);">
      <span style="color: var(--color-primary);">🤖</span>
      <span style="font-weight: 600; color: var(--text-primary);">
        Live Agents: {sim.liveAgents.length}
      </span>
    </div>
  {/if}
</div>
