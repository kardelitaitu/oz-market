<script lang="ts">
  import { rateLimits } from '$lib/stores/rateLimit.svelte';
  import { onMount } from 'svelte';

  let { action = 'create', showAlways = false }: { action: string; showAlways?: boolean } = $props();

  let info = $derived(rateLimits.forAction(action));
  let pct = $derived(info && info.limit > 0 ? info.remaining / info.limit : 1);
  let exhausted = $derived(info?.remaining === 0);
  let label = $derived(info ? `${info.remaining}/${info.limit}` : '');
  let show = $derived(showAlways || (info !== undefined && pct < 0.5));

  let colorClass = $derived.by(() => {
    if (exhausted) return 'exhausted';
    if (pct < 0.1) return 'critical';
    if (pct < 0.2) return 'low';
    if (pct < 0.5) return 'moderate';
    return 'ok';
  });
</script>

{#if show}
  <span class="badge {colorClass}" title="{action}: {label} remaining">
    <span class="action-label">{action}</span>
    {#if exhausted}
      <span class="remaining exhausted-label">waiting for reset…</span>
    {:else if info}
      <span class="remaining">{info!.remaining} / {info!.limit}</span>
    {/if}
  </span>
{/if}

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 0.7rem;
    font-weight: 600;
    white-space: nowrap;
    transition: all 0.2s ease;
  }
  .badge.ok {
    background: #dcfce7;
    color: #166534;
  }
  .badge.moderate {
    background: #fef9c3;
    color: #854d0e;
  }
  .badge.low {
    background: #fed7aa;
    color: #9a3412;
  }
  .badge.critical {
    background: #fecaca;
    color: #991b1b;
  }
  .badge.exhausted {
    background: #fecaca;
    color: #7f1d1d;
    animation: pulse-red 1.5s ease-in-out infinite;
  }
  .action-label {
    text-transform: capitalize;
  }
  .remaining {
    opacity: 0.85;
  }
  .exhausted-label {
    font-style: italic;
    font-size: 0.65rem;
  }
  @keyframes pulse-red {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.55; }
  }
</style>
