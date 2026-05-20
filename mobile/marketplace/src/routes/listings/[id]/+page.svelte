<script lang="ts">
  import { getListing, openNegotiation } from '$lib/api/commands';
  import type { ListingSummary } from '$lib/api/commands';
  import { generateIdempotencyKey } from '$lib/utils/idempotency';
  import { rateLimits } from '$lib/stores/rateLimit.svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { isLoggedIn } from '$lib/stores/auth';

  let listing = $state<ListingSummary | null>(null);
  let loading = $state(true);
  let error = $state('');
  let offerAmount = $state(0);
  let offerCurrency = $state('USD');
  let offerSubmitting = $state(false);
  let offerError = $state('');

  const id = $derived($page.params.id);
  let negotiateExhausted = $derived(rateLimits.forAction('negotiate')?.remaining === 0);

  $effect(() => {
    if (id) {
      loadListing();
    }
  });

  async function loadListing() {
    loading = true;
    error = '';
    try {
      listing = await getListing(id);
      if (listing) {
        offerAmount = listing.listing.price.amount;
        offerCurrency = listing.listing.price.currency;
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleOpenNegotiation() {
    offerSubmitting = true;
    offerError = '';
    try {
      const result = await openNegotiation({
        listingId: id,
        currency: offerCurrency,
        amount: offerAmount,
        idempotencyKey: generateIdempotencyKey(),
      });
      goto(`/negotiations/${result.negotiation_id}`);
    } catch (e) {
      offerError = String(e);
    } finally {
      offerSubmitting = false;
    }
  }
</script>

{#if loading}
  <p>Loading listing...</p>
{:else if error}
  <p class="error">{error}</p>
  <button onclick={loadListing}>Retry</button>
{:else if listing}
  <h1>{listing.listing.title}</h1>

  <div class="field">
    <span class="label">Price</span>
    <span class="value">{listing.listing.price.currency} {listing.listing.price.amount}</span>
  </div>

  <div class="field">
    <span class="label">Status</span>
    <span class="value status-{listing.status}">{listing.status}</span>
  </div>

  <div class="field">
    <span class="label">Type</span>
    <span class="value">{listing.listing.listing_type}</span>
  </div>

  <div class="field">
    <span class="label">Location</span>
    <span class="value">{listing.listing.location.city}, {listing.listing.location.country_name}</span>
  </div>

  <div class="field">
    <span class="label">Description</span>
    <p>{listing.listing.description}</p>
  </div>

  {#if listing.seller_name}
    <div class="field">
      <span class="label">Seller</span>
      <span class="value">{listing.seller_name}</span>
    </div>
  {/if}

  <div class="actions">
    <a href="/listings/search">Back to Search</a>
  </div>

  {#if isLoggedIn() && listing.status === 'active'}
    <hr />
    <h2>Start Negotiation</h2>
    <div class="negotiation-form">
      <label>
        Currency
        <input type="text" bind:value={offerCurrency} />
      </label>
      <label>
        Offer Amount
        <input type="number" bind:value={offerAmount} min="0" step="0.01" />
      </label>
      <button onclick={handleOpenNegotiation} disabled={offerSubmitting || negotiateExhausted}>
        {offerSubmitting ? 'Opening...' : negotiateExhausted ? 'Rate Limited' : 'Open Negotiation'}
      </button>
      {#if offerError}
        <p class="error">{offerError}</p>
      {/if}
      {#if negotiateExhausted}
        <p class="rate-note">Negotiation rate limit reached ({rateLimits.forAction('negotiate')!.limit} opens/min). Wait and retry.</p>
      {/if}
    </div>
  {/if}
{/if}

<style>
  .rate-note {
    font-size: 0.8rem;
    color: #92400e;
    background: #fef3c7;
    padding: 6px 10px;
    border-radius: 4px;
    margin-top: 4px;
  }
  .field {
    margin: 12px 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .label {
    font-size: 0.8rem;
    color: var(--color-text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .value {
    font-size: 1rem;
  }
  p {
    margin: 0;
    line-height: 1.6;
    color: var(--color-text);
  }
  .error {
    color: var(--color-error);
  }
  .actions {
    margin-top: 24px;
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
  hr {
    margin: 24px 0;
    border: none;
    border-top: 1px solid var(--color-border);
  }
  .negotiation-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 300px;
  }
  .negotiation-form label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.85rem;
    color: var(--color-text-secondary);
  }
  .negotiation-form input {
    padding: 8px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
  }
  .negotiation-form button {
    padding: 10px 16px;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-weight: 500;
  }
  .negotiation-form button:disabled {
    opacity: 0.6;
  }
</style>
