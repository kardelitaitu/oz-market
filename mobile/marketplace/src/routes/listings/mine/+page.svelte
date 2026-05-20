<script lang="ts">
  import { myListings } from '$lib/api/commands';
  import type { ListingSummary } from '$lib/api/commands';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  let listings = $state<ListingSummary[]>([]);
  let loading = $state(true);
  let error = $state('');
  let nextCursor = $state<string | undefined>();

  onMount(() => loadListings());

  async function loadListings() {
    loading = true;
    error = '';
    listings = [];
    nextCursor = undefined;

    try {
      const result = await myListings({ limit: 50 });
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
      const result = await myListings({ limit: 50, cursor: nextCursor });
      listings = [...listings, ...result.items];
      nextCursor = result.next_cursor;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<h1>My Listings</h1>

<button class="refresh" onclick={loadListings} disabled={loading}>
  {loading ? 'Refreshing...' : 'Refresh'}
</button>

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
      <p class="empty">You haven't created any listings yet.</p>
    {/if}
  {/each}
</div>

{#if nextCursor}
  <button class="load-more" onclick={loadMore} disabled={loading}>
    {loading ? 'Loading...' : 'Load More'}
  </button>
{/if}

<style>
  .refresh {
    margin-bottom: 12px;
    padding: 8px 16px;
    background: var(--color-bg-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    cursor: pointer;
    font-weight: 500;
  }
  .refresh:disabled {
    opacity: 0.6;
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
