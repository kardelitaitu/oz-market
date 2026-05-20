<script lang="ts">
  import { onMount } from 'svelte';
  import { health } from '$lib/api/commands';
  import { isLoggedIn, checkAuth } from '$lib/stores/auth';
  import { goto } from '$app/navigation';

  let serverStatus = $state<'checking' | 'ok' | 'error'>('checking');
  let statusMessage = $state('');

  onMount(async () => {
    await checkAuth();
    try {
      const result = await health();
      serverStatus = 'ok';
      statusMessage = JSON.stringify(result);
    } catch (e) {
      serverStatus = 'error';
      statusMessage = String(e);
    }
  });
</script>

<h1>Marketplace</h1>

<div class="status-card">
  <h2>Server Status</h2>
  {#if serverStatus === 'checking'}
    <p>Connecting to server...</p>
  {:else if serverStatus === 'ok'}
    <p class="ok">Connected</p>
    <pre>{statusMessage}</pre>
  {:else}
    <p class="error">Failed to connect</p>
    <pre>{statusMessage}</pre>
    <p class="hint">Go to <a href="/settings">Settings</a> to configure the backend URL.</p>
  {/if}
</div>

<div class="actions">
  {#if isLoggedIn()}
    <a href="/listings/search">Search Listings</a>
    <a href="/listings/create">Create Listing</a>
  {:else}
    <a href="/login">Login</a>
  {/if}
</div>

<style>
  .status-card {
    background: var(--color-bg-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    padding: 16px;
    margin: 16px 0;
    box-shadow: var(--shadow);
  }
  .ok {
    color: var(--color-success);
    font-weight: 600;
  }
  .error {
    color: var(--color-error);
    font-weight: 600;
  }
  pre {
    font-size: 0.8rem;
    overflow-x: auto;
    background: #f1f5f9;
    padding: 8px;
    border-radius: 4px;
  }
  .hint {
    font-size: 0.85rem;
    margin-top: 8px;
  }
  .hint a {
    color: var(--color-primary);
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .actions a {
    display: inline-block;
    padding: 8px 16px;
    background: var(--color-primary);
    color: white;
    text-decoration: none;
    border-radius: var(--radius);
    font-weight: 500;
  }
</style>
