<script>
  /**
   * @typedef {'buyer'|'seller'} AgentRole
   */
  let { name = 'Agent', role = 'buyer', isActive = false, title = '' } = $props();

  const accentColor = $derived(role === 'buyer' ? 'var(--color-primary)' : 'var(--color-secondary)');
  const glowVar    = $derived(role === 'buyer' ? 'var(--color-primary-glow)' : 'var(--color-secondary-glow)');
  const borderClr  = $derived(isActive ? accentColor : 'rgba(255,255,255,0.08)');
  const shadow     = $derived(isActive ? `0 0 14px ${glowVar}` : 'none');
</script>

<style>
  .agent-card {
    background: var(--bg-card);
    border-radius: 12px;
    padding: 1.5rem;
    min-width: 160px;
    max-width: 220px;
    text-align: center;
    overflow: hidden;
    transition: border-color 0.4s ease;
  }

  .agent-card .avatar {
    font-size: 2.5rem;
    margin-bottom: 0.5rem;
  }

  .agent-card h4 {
    font-family: var(--font-heading);
    font-size: 1rem;
    word-wrap: break-word;
    text-overflow: ellipsis;
    white-space: nowrap;
    overflow: hidden;
  }

  .agent-card p {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-bottom: 0;
  }
</style>

<div
  class="agent-card"
  style="border: 1px solid {borderClr}; box-shadow: {shadow};"
>
  <div class="avatar">🤖</div>
  <h4
    style="color: {accentColor};"
    title={name}
  >{name}</h4>
  <p>{title}</p>
</div>
