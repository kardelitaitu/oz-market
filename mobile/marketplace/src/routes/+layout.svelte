<script lang="ts">
  import '../app.css';
  import { isLoggedIn } from '$lib/stores/auth';
  import { rateLimits } from '$lib/stores/rateLimit.svelte';
  import { page } from '$app/stores';
  import { onMount, onDestroy } from 'svelte';

  let { children } = $props();

  onMount(() => rateLimits.startPolling(4000));
  onDestroy(() => rateLimits.stopPolling());

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
</style>
