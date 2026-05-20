<script lang="ts">
  import { onMount } from 'svelte';
  import { agentQuery, type AgentQueryResponse } from '$lib/api/commands';
  import { checkAuth } from '$lib/stores/auth';
  import { rateLimits } from '$lib/stores/rateLimit.svelte';
  import { goto } from '$app/navigation';

  let messages = $state<{ role: 'user' | 'agent'; text: string }[]>([]);
  let input = $state('');
  let conversationId = $state<string | undefined>(undefined);
  let loading = $state(false);
  let listingResults = $state<AgentQueryResponse['listing_ids'] | null>(null);

  let agentExhausted = $derived(rateLimits.forAction('agent')?.remaining === 0);

  onMount(() => {
    checkAuth();
    messages.push({
      role: 'agent',
      text: 'Hello! I can help you find listings. Try saying things like "show me laptops under $1000" or "find used phones in Seattle".',
    });
  });

  async function send() {
    const q = input.trim();
    if (!q || loading) return;
    input = '';
    messages.push({ role: 'user', text: q });
    loading = true;

    try {
      const resp = await agentQuery({ query: q, conversationId });
      messages.push({ role: 'agent', text: resp.message });
      conversationId = resp.conversation_id;
      if (resp.listing_ids && resp.listing_ids.length > 0) {
        listingResults = resp.listing_ids;
      }
      if (resp.actions.length > 0) {
        for (const action of resp.actions) {
          messages.push({
            role: 'agent',
            text: `→ ${action.label}`,
          });
        }
      }
    } catch (e) {
      messages.push({ role: 'agent', text: `Error: ${e}` });
    } finally {
      loading = false;
    }
  }

  function handleAction(actionType: string, params: Record<string, unknown>) {
    if (actionType === 'search') {
      const searchParams: Record<string, string> = {};
      if (params.query) searchParams.query = String(params.query);
      if (params.category) searchParams.category = String(params.category);
      if (params.listing_type) searchParams.listing_type = String(params.listing_type);
      const qs = new URLSearchParams(searchParams).toString();
      goto(`/listings/search?${qs}`);
    } else if (actionType === 'view_listing' && params.listing_id) {
      goto(`/listings/${params.listing_id}`);
    }
  }
</script>

<h1>AI Agent</h1>

<div class="chat">
  {#each messages as msg, i}
    <div class="message {msg.role}">
      <span class="role-tag">{msg.role === 'user' ? 'You' : 'Agent'}</span>
      <p>{msg.text}</p>
      {#if msg.role === 'agent' && msg.text.startsWith('→ ')}
        <button onclick={() => {
          const listingIds = listingResults;
          if (listingIds && listingIds.length > 0) {
            goto(`/listings/${listingIds[0]}`);
          }
        }}>
          View
        </button>
      {/if}
    </div>
  {/each}
  {#if loading}
    <div class="message agent">
      <span class="role-tag">Agent</span>
      <p>Thinking...</p>
    </div>
  {/if}
</div>

<div class="input-row">
  <input
    type="text"
    bind:value={input}
    placeholder="Ask about listings..."
    disabled={loading}
    onkeydown={(e) => { if (e.key === 'Enter') send(); }}
  />
  <button onclick={send} disabled={loading || agentExhausted}>
    {agentExhausted ? 'Rate Limited' : loading ? 'Sending...' : 'Send'}
  </button>
</div>

{#if agentExhausted}
  <div class="rate-warning">
    Agent query rate limit exhausted ({rateLimits.forAction('agent')!.limit} queries/min).
    Wait a moment before sending another message.
  </div>
{/if}

<style>
  .chat {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 16px 0;
    max-height: 60vh;
    overflow-y: auto;
  }
  .message {
    background: var(--color-bg-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    padding: 12px;
  }
  .message.user {
    background: var(--color-primary);
    color: white;
    border-color: var(--color-primary);
    align-self: flex-end;
  }
  .role-tag {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    opacity: 0.7;
  }
  .message p {
    margin: 4px 0 0;
  }
  .input-row {
    display: flex;
    gap: 8px;
    position: sticky;
    bottom: 0;
    background: var(--color-bg);
    padding: 8px 0;
  }
  .input-row input {
    flex: 1;
    padding: 8px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
  }
  .input-row button {
    padding: 8px 16px;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
  }
  .input-row button:disabled {
    opacity: 0.5;
  }
</style>
