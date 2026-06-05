<script>
  let currentTheme = $state(localStorage.getItem('oz-market-theme') || 'midnight');

  const themes = [
    { id: 'midnight', label: 'Midnight' },
    { id: 'emerald',  label: 'Emerald'  },
    { id: 'crimson',  label: 'Crimson'  },
    { id: 'solar',    label: 'Solar'    },
    { id: 'nordic',   label: 'Nordic'   },
  ];

  function setTheme(id) {
    currentTheme = id;
  }

  $effect(() => {
    document.body.setAttribute('data-theme', currentTheme);
    localStorage.setItem('oz-market-theme', currentTheme);
  });
</script>

<style>
  .theme-swatches {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 30px;
    padding: 0.4rem 0.75rem;
    transition: background-color 0.3s ease, border-color 0.3s ease, box-shadow 0.3s ease;
  }

  .theme-swatches:hover {
    border-color: var(--color-primary);
    background: rgba(255, 255, 255, 0.06);
    box-shadow: 0 0 12px var(--color-primary-glow);
  }

  .theme-swatches-label {
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.8px;
    padding-right: 0.25rem;
    white-space: nowrap;
  }

  .theme-swatch {
    position: relative;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.15);
    cursor: pointer;
    padding: 0;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.2s ease, border-color 0.2s ease, box-shadow 0.2s ease;
    flex-shrink: 0;
  }

  .theme-swatch::before {
    content: '';
    position: absolute;
    inset: 2px;
    border-radius: 50%;
    background: var(--swatch-color);
    transition: transform 0.2s ease;
  }

  .theme-swatch:hover {
    transform: scale(1.25);
    border-color: rgba(255, 255, 255, 0.5);
    box-shadow: 0 0 10px var(--swatch-color);
    z-index: 1;
  }

  .theme-swatch.active {
    border-color: white;
    box-shadow: 0 0 0 2px var(--swatch-color), 0 0 12px var(--swatch-color);
    transform: scale(1.15);
  }

  .theme-swatch.active::before {
    transform: scale(0.85);
  }

  .swatch-midnight { --swatch-color: HSL(263, 90%, 65%); }
  .swatch-emerald  { --swatch-color: HSL(142, 80%, 50%); }
  .swatch-crimson  { --swatch-color: HSL(342, 85%, 60%); }
  .swatch-solar    { --swatch-color: HSL(38, 95%, 55%); }
  .swatch-nordic   { --swatch-color: HSL(200, 85%, 60%); }

  @media (max-width: 600px) {
    .theme-swatches {
      justify-self: end;
    }
    .theme-swatches-label {
      display: none;
    }
  }

  @media (max-width: 400px) {
    .theme-swatches {
      width: 100%;
      justify-content: center;
      align-self: center;
    }
  }
</style>

<div class="theme-swatches" role="group" aria-label="Select Theme">
  <span class="theme-swatches-label">Theme</span>
  {#each themes as th}
    <button
      class="theme-swatch swatch-{th.id} {currentTheme === th.id ? 'active' : ''}"
      aria-label="{th.label} theme"
      aria-pressed={currentTheme === th.id}
      title={th.label}
      onclick={() => setTheme(th.id)}
    ></button>
  {/each}
</div>
