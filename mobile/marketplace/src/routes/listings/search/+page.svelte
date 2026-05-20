<script lang="ts">
  import { searchListings } from '$lib/api/commands';
  import type { ListingSummary } from '$lib/api/commands';
  import { rateLimits } from '$lib/stores/rateLimit.svelte';
  import { goto } from '$app/navigation';

  let query = $state('');
  let listings = $state<ListingSummary[]>([]);
  let loading = $state(false);
  let error = $state('');
  let nextCursor = $state<string | undefined>();

  let searchExhausted = $derived(rateLimits.forAction('search')?.remaining === 0);

  async function handleSearch() {
    loading = true;
    error = '';
    listings = [];
    nextCursor = undefined;

    try {
      const result = await searchListings({ query: query || undefined, limit: 20 });
      listings = result.items;
      nextCursor = result.next_cursor;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function loadMore() {
    if (!nextCursor || loading) return;
    loading = true;
    try {
      const result = await searchListings({ query: query || undefined, limit: 20, cursor: nextCursor });
      listings = [...listings, ...result.items];
      nextCursor = result.next_cursor;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<h1>Search Listings</h1>

{#if searchExhausted}
  <div class="rate-warning">
    Search rate limit exhausted. Wait a moment and try again.
  </div>
{:else if rateLimits.forAction('search') && rateLimits.forAction('search')!.remaining < 10}
  <div class="rate-note">
    {rateLimits.forAction('search')!.remaining} searches left this minute.
  </div>
{/if}

<div class="search-bar">
  <input
    type="text"
    bind:value={query}
    placeholder="Search laptops, phones, services..."
    onkeydown={(e) => { if (e.key === 'Enter') handleSearch(); }}
  />
  <button onclick={handleSearch} disabled={loading}>
    {loading ? 'Searching...' : 'Search'}
  </button>
</div>

{#if error}
  <p class="error">{error}</p>
{/if}

<div class="results">
  {#each listings as item (item.listing_id)}
    <div class="card" onclick={() => goto(`/listings/${item.listing_id}`)} onkeypress={() => goto(`/listings/${item.listing_id}`)} role="button" tabindex="0">
      <div class="card-title">{item.listing.title}</div>
      <div class="card-meta">
        <span class="price">{item.listing.price.currency} {item.listing.price.amount}</span>
        <span class="status">{item.status}</span>
        <span class="location">{item.listing.location.city}, {item.listing.location.country_code}</span>
      </div>
    </div>
  {:else}
    {#if !loading && !error}
      <p class="empty">No results. Try a different search term.</p>
    {/if}
  {/each}
</div>

{#if nextCursor}
  <button class="load-more" onclick={loadMore} disabled={loading}>
    {loading ? 'Loading...' : 'Load More'}
  </button>
{/if}

<style>
  .search-bar {
    display: flex;
    gap: 8px;
    margin-bottom: 16px;
  }
  .search-bar input {
    flex: 1;
    padding: 10px 14px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    font-size: 1rem;
  }
  .search-bar button {
    padding: 10px 20px;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius);
    font-weight: 500;
    cursor: pointer;
  }
  .search-bar button:disabled {
    opacity: 0.6;
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
  .rate-note {
    background: #fef9c3;
    color: #854d0e;
    padding: 6px 12px;
    border-radius: var(--radius);
    font-size: 0.8rem;
    margin-bottom: 12px;
  }
  .results {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .card {
    padding: 12px 16px;
    background: var(--color-bg-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    cursor: pointer;
    box-shadow: var(--shadow);
  }
  .card-title {
    font-weight: 600;
    margin-bottom: 4px;
  }
  .card-meta {
    display: flex;
    gap: 12px;
    font-size: 0.85rem;
    color: var(--color-text-secondary);
  }
  .price {
    color: var(--color-primary);
    font-weight: 500;
  }
  .status {
    text-transform: uppercase;
    font-size: 0.75rem;
  }
  .empty {
    color: var(--color-text-secondary);
    text-align: center;
    padding: 32px;
  }
  .load-more {
    display: block;
    width: 100%;
    margin-top: 12px;
    padding: 10px;
    background: var(--color-bg-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    cursor: pointer;
    font-weight: 500;
  }
  .error {
    color: var(--color-error);
  }
</style>
