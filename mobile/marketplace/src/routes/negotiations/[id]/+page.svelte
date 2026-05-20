<script lang="ts">
  import {
    getNegotiation,
    submitOffer,
    acceptNegotiation,
    rejectNegotiation,
    requestContactReveal,
    approveContactReveal,
  } from '$lib/api/commands';
  import type { NegotiationResponse, NegotiationHistoryEntry } from '$lib/api/commands';
  import { generateIdempotencyKey } from '$lib/utils/idempotency';
  import { rateLimits } from '$lib/stores/rateLimit.svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';

  let negotiation = $state<NegotiationResponse | null>(null);
  let loading = $state(true);
  let error = $state('');
  let polling = $state(false);
  let counterAmount = $state(0);
  let counterCurrency = $state('USD');
  let actionSubmitting = $state('');
  let actionError = $state('');
  let pendingRevealId = $state<string | undefined>(undefined);

  const id = $derived($page.params.id);
  let pollTimer: ReturnType<typeof setInterval> | undefined;

  let offerExhausted = $derived(rateLimits.forAction('offer')?.remaining === 0);
  let acceptExhausted = $derived(rateLimits.forAction('accept')?.remaining === 0);
  let rejectExhausted = $derived(rateLimits.forAction('reject')?.remaining === 0);
  let revealExhausted = $derived(rateLimits.forAction('reveal')?.remaining === 0);
  let approveExhausted = $derived(rateLimits.forAction('approve')?.remaining === 0);

  onMount(() => {
    loadNegotiation();
    pollTimer = setInterval(loadNegotiation, 5000);
    return () => { if (pollTimer) clearInterval(pollTimer); };
  });

  async function loadNegotiation() {
    if (!loading) polling = true;
    error = '';
    try {
      const result = await getNegotiation(id);
      negotiation = result;
      pendingRevealId = result.reveal_id;
      const latest = result.offer_history.at(-1);
      if (latest) {
        counterAmount = latest.offer_amount;
        counterCurrency = latest.offer_currency;
      }
    } catch (e) {
      if (!loading) error = String(e);
    } finally {
      loading = false;
      polling = false;
    }
  }

  async function handleSubmitOffer() {
    actionSubmitting = 'offer';
    actionError = '';
    try {
      const result = await submitOffer({
        negotiationId: id,
        currency: counterCurrency,
        amount: counterAmount,
        idempotencyKey: generateIdempotencyKey(),
      });
      negotiation = result;
    } catch (e) {
      actionError = String(e);
    } finally {
      actionSubmitting = '';
    }
  }

  async function handleAccept() {
    actionSubmitting = 'accept';
    actionError = '';
    try {
      const result = await acceptNegotiation(id, generateIdempotencyKey());
      negotiation = result;
    } catch (e) {
      actionError = String(e);
    } finally {
      actionSubmitting = '';
    }
  }

  async function handleReject() {
    actionSubmitting = 'reject';
    actionError = '';
    try {
      const result = await rejectNegotiation(id, generateIdempotencyKey());
      negotiation = result;
    } catch (e) {
      actionError = String(e);
    } finally {
      actionSubmitting = '';
    }
  }

  async function handleRequestReveal() {
    actionSubmitting = 'reveal';
    actionError = '';
    try {
      const result = await requestContactReveal(id, generateIdempotencyKey());
      pendingRevealId = result.reveal_id;
      await loadNegotiation();
    } catch (e) {
      actionError = String(e);
    } finally {
      actionSubmitting = '';
    }
  }

  async function handleApproveReveal() {
    if (!pendingRevealId) return;
    actionSubmitting = 'approve_reveal';
    actionError = '';
    try {
      await approveContactReveal(pendingRevealId, generateIdempotencyKey());
      await loadNegotiation();
    } catch (e) {
      actionError = String(e);
    } finally {
      actionSubmitting = '';
    }
  }

  function needsReveal(s: string) {
    return s === 'contact_requested' || s === 'reserved' || s === 'near_close';
  }
</script>

{#if loading}
  <p>Loading negotiation...</p>
{:else if error}
  <p class="error">{error}</p>
  <button onclick={loadNegotiation}>Retry</button>
{:else if negotiation}
  <h1>Negotiation</h1>

  <div class="status-bar status-{negotiation.status}">
    Status: {negotiation.status}
    {#if polling}
      <span class="polling-indicator">(polling...)</span>
    {/if}
  </div>

  <div class="field">
    <span class="label">Latest Offer</span>
    <span class="value">{negotiation.offer_currency} {negotiation.latest_offer_amount}</span>
  </div>

  <h2>Offer History</h2>
  <div class="history">
    {#each negotiation.offer_history as entry (entry.entry_id)}
      <div class="entry entry-{entry.entry_type}">
        <div class="entry-type">{entry.entry_type}</div>
        <div class="entry-amount">{entry.offer_currency} {entry.offer_amount}</div>
        <div class="entry-actor">{entry.actor_role}: {entry.actor_subject}</div>
        <div class="entry-status">→ {entry.resulting_status}</div>
        <div class="entry-time">{entry.created_at}</div>
      </div>
    {:else}
      <p class="empty">No offers yet.</p>
    {/each}
  </div>

  <!-- Actions based on status -->
  {#if negotiation.status === 'open' || negotiation.status === 'countered'}
    <hr />
    <h2>Submit Counter-Offer</h2>
    <div class="action-form">
      <label>
        Currency
        <input type="text" bind:value={counterCurrency} />
      </label>
      <label>
        Amount
        <input type="number" bind:value={counterAmount} min="0" step="0.01" />
      </label>
      <button onclick={handleSubmitOffer} disabled={actionSubmitting === 'offer' || offerExhausted}>
        {actionSubmitting === 'offer' ? 'Submitting...' : offerExhausted ? 'Rate Limited' : 'Submit Offer'}
      </button>
      {#if offerExhausted}
        <p class="rate-note">Offer rate limit reached. Wait before submitting another offer.</p>
      {/if}
    </div>

    <div class="action-buttons">
      <button class="accept" onclick={handleAccept} disabled={actionSubmitting === 'accept' || acceptExhausted}>
        {actionSubmitting === 'accept' ? 'Accepting...' : acceptExhausted ? 'Rate Limited' : 'Accept Offer'}
      </button>
      <button class="reject" onclick={handleReject} disabled={actionSubmitting === 'reject' || rejectExhausted}>
        {actionSubmitting === 'reject' ? 'Rejecting...' : rejectExhausted ? 'Rate Limited' : 'Reject'}
      </button>
    </div>
    {#if acceptExhausted || rejectExhausted}
      <p class="rate-note">Rate limited. Wait before taking this action.</p>
    {/if}
  {/if}

  {#if needsReveal(negotiation.status)}
    <hr />
    <button class="reveal" onclick={handleRequestReveal} disabled={actionSubmitting === 'reveal' || revealExhausted}>
      {actionSubmitting === 'reveal' ? 'Requesting...' : revealExhausted ? 'Rate Limited' : 'Request Contact Reveal'}
    </button>
    {#if revealExhausted}
      <p class="rate-note">Reveal request rate limit reached. Wait before requesting.</p>
    {/if}
  {/if}

  {#if negotiation.status === 'contact_requested' && pendingRevealId}
    <hr />
    <button class="approve-reveal" onclick={handleApproveReveal} disabled={actionSubmitting === 'approve_reveal' || approveExhausted}>
      {actionSubmitting === 'approve_reveal' ? 'Approving...' : approveExhausted ? 'Rate Limited' : 'Approve Contact Reveal'}
    </button>
    {#if approveExhausted}
      <p class="rate-note">Approve rate limit reached. Wait before approving.</p>
    {/if}
  {/if}

  {#if actionError}
    <p class="error">{actionError}</p>
  {/if}

  <div class="back">
    <a href="/listings/search">Back to Search</a>
  </div>
{/if}

<style>
  .status-bar {
    padding: 8px 12px;
    border-radius: var(--radius);
    font-weight: 600;
    margin-bottom: 16px;
  }
  .status-open { background: #ebf8ff; color: #2b6cb0; }
  .status-countered { background: #fefcbf; color: #975a16; }
  .status-reserved { background: #c6f6d5; color: #276749; }
  .status-near_close { background: #c6f6d5; color: #276749; }
  .status-closed { background: #e2e8f0; color: #4a5568; }
  .status-cancelled { background: #fed7d7; color: #9b2c2c; }
  .status-contact_requested { background: #e9d8fd; color: #6b46c1; }
  .status-contact_revealed { background: #c6f6d5; color: #276749; }
  .polling-indicator {
    font-size: 0.75rem;
    font-weight: 400;
    opacity: 0.7;
  }
  .field {
    margin: 8px 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .label {
    font-size: 0.8rem;
    color: var(--color-text-secondary);
    text-transform: uppercase;
  }
  .value {
    font-size: 1.1rem;
    font-weight: 600;
  }
  h2 {
    font-size: 1rem;
    margin: 20px 0 8px;
  }
  .history {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .entry {
    padding: 8px 12px;
    background: var(--color-bg-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    font-size: 0.85rem;
  }
  .entry-offer { border-left: 3px solid var(--color-primary); }
  .entry-accept { border-left: 3px solid #38a169; }
  .entry-reject { border-left: 3px solid #e53e3e; }
  .entry-type {
    font-weight: 600;
    text-transform: uppercase;
    font-size: 0.75rem;
  }
  .entry-amount { font-size: 1rem; }
  .entry-actor { font-size: 0.75rem; color: var(--color-text-secondary); }
  .entry-status { font-size: 0.75rem; color: var(--color-text-secondary); }
  .entry-time { font-size: 0.7rem; color: var(--color-text-secondary); }
  .empty { color: var(--color-text-secondary); }
  hr { margin: 16px 0; border: none; border-top: 1px solid var(--color-border); }
  .action-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 12px;
  }
  .action-form label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.85rem;
    color: var(--color-text-secondary);
  }
  .action-form input {
    padding: 8px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
  }
  .action-form button {
    padding: 10px 16px;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-weight: 500;
  }
  .action-buttons {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  .action-buttons button {
    flex: 1;
    padding: 10px 16px;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-weight: 500;
  }
  .accept { background: #38a169; color: white; }
  .reject { background: #e53e3e; color: white; }
  .reveal {
    padding: 10px 16px;
    background: #6b46c1;
    color: white;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-weight: 500;
  }
  .approve-reveal {
    padding: 10px 16px;
    background: #38a169;
    color: white;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-weight: 500;
  }
  button:disabled { opacity: 0.6; }
  .error { color: var(--color-error); margin-top: 8px; }
  .rate-note {
    font-size: 0.8rem;
    color: #92400e;
    background: #fef3c7;
    padding: 6px 10px;
    border-radius: 4px;
    margin-top: 4px;
  }
  .back { margin-top: 24px; }
  .back a {
    display: inline-block;
    padding: 8px 16px;
    background: var(--color-primary);
    color: white;
    text-decoration: none;
    border-radius: var(--radius);
  }
</style>
