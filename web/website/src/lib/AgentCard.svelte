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

<div
  style="background: var(--bg-card); border: 1px solid {borderClr}; padding: 1.5rem; border-radius: 12px; min-width: 160px; text-align: center; max-width: 220px; overflow: hidden; transition: border-color 0.4s ease; box-shadow: {shadow};"
>
  <div style="font-size: 2.5rem; margin-bottom: 0.5rem;">🤖</div>
  <h4
    style="font-family: var(--font-heading); font-size: 1rem; color: {accentColor}; word-wrap: break-word; text-overflow: ellipsis; white-space: nowrap; overflow: hidden;"
    title={name}
  >{name}</h4>
  <p style="font-size: 0.8rem; color: var(--text-muted); margin-bottom: 0;">{title}</p>
</div>
