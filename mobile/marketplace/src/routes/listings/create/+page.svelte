<script lang="ts">
  import { createListing } from '$lib/api/commands';
  import { generateIdempotencyKey } from '$lib/utils/idempotency';
  import { rateLimits } from '$lib/stores/rateLimit.svelte';
  import { goto } from '$app/navigation';

  let title = $state('');
  let description = $state('');
  let currency = $state('USD');
  let amount = $state(0);
  let countryCode = $state('US');
  let city = $state('');
  let listingType = $state<'product' | 'service' | 'property'>('product');
  let submitting = $state(false);
  let error = $state('');

  let createInfo = $derived(rateLimits.forAction('create'));
  let createExhausted = $derived(createInfo?.remaining === 0);

  async function handleSubmit(e: Event) {
    e.preventDefault();
    submitting = true;
    error = '';
    try {
      await createListing({
        title,
        description,
        listingType,
        currency,
        amount,
        countryCode,
        city,
        idempotencyKey: generateIdempotencyKey(),
      });
      goto('/listings/search');
    } catch (err) {
      error = String(err);
    } finally {
      submitting = false;
    }
  }
</script>

<h1>Create Listing</h1>

{#if createExhausted}
  <div class="rate-warning">
    Create rate limit exhausted. You can create up to {createInfo!.limit} listings per minute.
    Wait a moment and try again.
  </div>
{/if}

<form onsubmit={handleSubmit}>
  <label>
    Type
    <select bind:value={listingType}>
      <option value="product">Product</option>
      <option value="service">Service</option>
      <option value="property">Property</option>
    </select>
  </label>

  <label>
    Title
    <input type="text" bind:value={title} required />
  </label>

  <label>
    Description
    <textarea bind:value={description} rows={4} required></textarea>
  </label>

  <div class="row">
    <label>
      Currency
      <input type="text" bind:value={currency} placeholder="USD" />
    </label>
    <label>
      Amount
      <input type="number" bind:value={amount} min="0" step="0.01" required />
    </label>
  </div>

  <div class="row">
    <label>
      Country
      <input type="text" bind:value={countryCode} placeholder="US" />
    </label>
    <label>
      City
      <input type="text" bind:value={city} required />
    </label>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <button type="submit" disabled={submitting}>
    {submitting ? 'Creating...' : 'Create Listing'}
  </button>
</form>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 500px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 0.9rem;
    color: var(--color-text-secondary);
  }
  input, select, textarea {
    padding: 8px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    font-size: 1rem;
    font-family: inherit;
  }
  .row {
    display: flex;
    gap: 12px;
  }
  .row label {
    flex: 1;
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
    color: var(--color-error, #e53e3e);
    font-size: 0.9rem;
  }
  .rate-warning {
    background: #fecaca;
    color: #7f1d1d;
    padding: 8px 12px;
    border-radius: var(--radius);
    font-size: 0.85rem;
    margin-bottom: 12px;
    font-weight: 500;
  }
</style>
