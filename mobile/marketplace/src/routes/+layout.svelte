<script lang="ts">
  import '../app.css';
  import { isLoggedIn } from '$lib/stores/auth';
  import { rateLimits } from '$lib/stores/rateLimit.svelte';
  import { page } from '$app/stores';
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { sendNotification } from '@tauri-apps/plugin-notification';

  let { children } = $props();

  let sseNotification: (() => void) | undefined;
  let sseStatus: (() => void) | undefined;
  let sseStatuses = $state<Record<string, string>>({});

  let anyConnected = $derived(
    Object.values(sseStatuses).some((s) => s === 'connected'),
  );
  let anyReconnecting = $derived(
    Object.values(sseStatuses).some((s) => s === 'reconnecting' || s === 'connecting'),
  );

  onMount(() => {
    rateLimits.startPolling();
    listen<{ event_type: string; data: string }>('negotiation-update', (event) => {
      try {
        const parsed = JSON.parse(event.payload.data);
        const status = parsed.status ?? 'updated';
        sendNotification({
          title: `Negotiation ${status}`,
          body: `Negotiation ${parsed.negotiation_id?.slice(0, 8)}… is now ${status}`,
        });
      } catch { /* ignore */ }
    }).then((u) => sseNotification = u);
    listen<{ negotiation_id: string; status: string }>('negotiation-listener-status', (event) => {
      const { negotiation_id, status } = event.payload;
      sseStatuses = { ...sseStatuses, [negotiation_id]: status };
    }).then((u) => sseStatus = u);
  });
  onDestroy(() => {
    rateLimits.stopPolling();
    sseNotification?.();
    sseStatus?.();
  });

  let showRateBar = $derived(rateLimits.anyExhausted || rateLimits.anyLow);
</script>

<nav>
  <a href="/" class="logo">Marketplace</a>
  <div class="nav-links">
    {#if isLoggedIn()}
      <a href="/listings/search" class={$page.url.pathname.startsWith('/listings/search') ? 'active' : ''}>Search</a>
      <a href="/listings/mine" class={$page.url.pathname === '/listings/mine' ? 'active' : ''}>My Listings</a>
      <a href="/listings/create" class={$page.url.pathname === '/listings/create' ? 'active' : ''}>Create</a>
      <a href="/agent" class={$page.url.pathname === '/agent' ? 'active' : ''}>Agent</a>
      <a href="/settings" class={$page.url.pathname === '/settings' ? 'active' : ''}>Settings</a>
    {:else}
      <a href="/login" class={$page.url.pathname === '/login' ? 'active' : ''}>Login</a>
    {/if}
    {#if anyConnected || anyReconnecting}
      <span class="sse-indicator" class:sse-connected={anyConnected} class:sse-reconnecting={anyReconnecting} title={anyConnected ? 'Live' : 'Reconnecting…'}>
        {anyConnected ? '●' : '◌'}
      </span>
    {/if}
  </div>
</nav>

{#if showRateBar}
  <div class="rate-bar">
    {#each rateLimits.all as l (l.action)}
      {#if l.remaining / l.limit < 0.5}
        <span class="rate-chip rate-{l.remaining === 0 ? 'exhausted' : l.remaining / l.limit < 0.1 ? 'critical' : l.remaining / l.limit < 0.2 ? 'low' : 'moderate'}">
          <span class="chip-action">{l.action}</span>
          {l.remaining === 0
            ? 'waiting for reset…'
            : `${l.remaining} / ${l.limit}`}
        </span>
      {/if}
    {/each}
  </div>
{/if}

<main>
  {#if children}
    {@render children()}
  {/if}
</main>

<style>
  nav {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 0;
    border-bottom: 1px solid var(--color-border);
    margin-bottom: 16px;
  }
  .logo {
    font-weight: 700;
    font-size: 1.1rem;
    text-decoration: none;
    color: var(--color-primary);
  }
  .nav-links {
    display: flex;
    gap: 8px;
    margin-left: auto;
  }
  .nav-links a {
    padding: 4px 12px;
    border-radius: var(--radius);
    text-decoration: none;
    color: var(--color-text-secondary);
    font-size: 0.9rem;
  }
  .nav-links a.active {
    background: var(--color-primary);
    color: white;
  }
  .sse-indicator {
    font-size: 0.8rem;
    margin-left: 4px;
  }
  .sse-connected { color: #38a169; }
  .sse-reconnecting { color: #d69e2e; animation: pulse 1s ease-in-out infinite; }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }
</style>
