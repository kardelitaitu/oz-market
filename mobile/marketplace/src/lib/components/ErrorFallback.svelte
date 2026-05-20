<script lang="ts">
  let {
    fallbackMessage = 'Something went wrong',
    onRetry,
  }: {
    fallbackMessage?: string;
    onRetry?: () => void;
  } = $props();
</script>

<div class="error-fallback">
  <h2>{fallbackMessage}</h2>
  {#if onRetry}
    <button onclick={onRetry}>
      Try Again
    </button>
  {/if}
</div>

<style>
  .error-fallback {
    padding: 24px;
    text-align: center;
    background: var(--color-bg-secondary);
    border: 1px solid var(--color-error);
    border-radius: var(--radius);
    margin: 16px 0;
  }
  button {
    padding: 8px 16px;
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
  }
  h2 {
    font-size: 1rem;
    margin-bottom: 8px;
  }
</style>
