<script lang="ts">
  import { login } from '$lib/api/commands';
  import { goto } from '$app/navigation';

  let sub = $state('');
  let sellerAccountId = $state('');
  let buyerAgentId = $state('');
  let error = $state('');
  let loading = $state(false);

  async function handleSubmit(e: Event) {
    e.preventDefault();
    loading = true;
    error = '';

    try {
      await login({
        sub: sub || 'agent-dev',
        sellerAccountId: sellerAccountId || undefined,
        buyerAgentId: buyerAgentId || undefined,
        roles: ['seller_listing_writer', 'buyer_negotiator', 'seller_contact_reveal_approver'],
        scopes: [
          'listing:create',
          'listing:read',
          'listing:search',
          'negotiation:create',
          'negotiation:read',
          'negotiation:offer:submit',
          'negotiation:reveal:request',
          'reveal:approve',
        ],
      });
      goto('/');
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<h1>Login</h1>

<form onsubmit={handleSubmit}>
  <label>
    Subject (sub)
    <input type="text" bind:value={sub} placeholder="agent-dev" />
  </label>
  <label>
    Seller Account ID
    <input type="text" bind:value={sellerAccountId} placeholder="optional" />
  </label>
  <label>
    Buyer Agent ID
    <input type="text" bind:value={buyerAgentId} placeholder="optional" />
  </label>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <button type="submit" disabled={loading}>
    {loading ? 'Logging in...' : 'Login'}
  </button>
</form>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 400px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.9rem;
    color: var(--color-text-secondary);
  }
  input {
    padding: 8px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    font-size: 1rem;
  }
  button {
    padding: 10px 16px;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius);
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.6;
  }
  .error {
    color: var(--color-error);
    font-size: 0.9rem;
  }
</style>
