<script lang="ts">
  import { getBaseUrl, setBaseUrl, logout } from '$lib/api/commands';
  import { setLoggedIn } from '$lib/stores/auth';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  let baseUrl = $state('');
  let saved = $state(false);

  onMount(async () => {
    baseUrl = await getBaseUrl();
  });

  async function handleSave() {
    await setBaseUrl(baseUrl);
    saved = true;
    setTimeout(() => saved = false, 2000);
  }

  async function handleLogout() {
    await logout();
    setLoggedIn(false);
    goto('/');
  }
</script>

<h1>Settings</h1>

<div class="section">
  <h2>Server</h2>
  <label>
    Backend URL
    <input type="text" bind:value={baseUrl} placeholder="http://127.0.0.1:3000" />
  </label>
  <button onclick={handleSave}>Save</button>
  {#if saved}
    <span class="saved">Saved</span>
  {/if}
</div>

<div class="section">
  <h2>Account</h2>
  <button class="logout" onclick={handleLogout}>Logout</button>
</div>

<style>
  .section {
    background: var(--color-bg-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    padding: 16px;
    margin: 16px 0;
  }
  h2 {
    margin: 0 0 12px 0;
    font-size: 1rem;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.9rem;
    color: var(--color-text-secondary);
    margin-bottom: 8px;
  }
  input {
    padding: 8px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    font-size: 0.95rem;
  }
  button {
    padding: 8px 16px;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius);
    font-weight: 500;
    cursor: pointer;
  }
  .logout {
    background: var(--color-error);
  }
  .saved {
    margin-left: 8px;
    color: var(--color-success);
    font-size: 0.9rem;
  }
</style>
