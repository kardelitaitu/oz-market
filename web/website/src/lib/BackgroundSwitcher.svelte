<script>
  let currentBg = $state(localStorage.getItem('oz-market-bg') || 'nebulae');

  const backgrounds = [
    { id: 'nebulae',   label: 'Nebulae'   },
    { id: 'honeycomb', label: 'Honeycomb' },
    { id: 'network',   label: 'Network'   },
    { id: 'circuit',   label: 'Circuit'   },
    { id: 'grid',      label: 'Grid'      },
    { id: 'cosmic',    label: 'Cosmic'    },
  ];

  function setBg(id) {
    currentBg = id;
  }

  $effect(() => {
    document.body.setAttribute('data-bg', currentBg);
    localStorage.setItem('oz-market-bg', currentBg);
  });
</script>

<style>
  .bg-swatches {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 30px;
    padding: 0.35rem 0.65rem;
  }

  .bg-swatches:hover {
    border-color: var(--color-primary);
    background: rgba(255, 255, 255, 0.06);
  }

  .bg-swatches-label {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.6px;
    padding-right: 0.15rem;
    white-space: nowrap;
  }

  .bg-swatch {
    position: relative;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.12);
    cursor: pointer;
    padding: 0;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.2s ease, border-color 0.2s ease;
    flex-shrink: 0;
  }

  .bg-swatch::before {
    content: '';
    position: absolute;
    inset: 2px;
    border-radius: 50%;
    background: var(--bg-color);
    transition: transform 0.2s ease;
  }

  .bg-swatch:hover {
    transform: scale(1.25);
    border-color: rgba(255, 255, 255, 0.5);
    z-index: 1;
  }

  .bg-swatch.active {
    border-color: white;
    box-shadow: 0 0 0 2px var(--bg-color), 0 0 10px var(--bg-color);
    transform: scale(1.15);
  }

  .bg-swatch.active::before {
    transform: scale(0.85);
  }

  .bg-nebulae   { --bg-color: HSL(263, 90%, 65%); }
  .bg-honeycomb { --bg-color: HSL(38, 95%, 55%); }
  .bg-network   { --bg-color: HSL(200, 85%, 60%); }
  .bg-circuit   { --bg-color: HSL(120, 60%, 45%); }
  .bg-grid      { --bg-color: HSL(0, 0%, 55%); }
  .bg-cosmic    { --bg-color: HSL(280, 100%, 60%); }

  @media (max-width: 400px) {
    .bg-swatches-label {
      display: none;
    }
  }
</style>

<div class="bg-swatches" role="group" aria-label="Select Background">
  <span class="bg-swatches-label">BG</span>
  {#each backgrounds as bg}
    <button
      class="bg-swatch bg-{bg.id} {currentBg === bg.id ? 'active' : ''}"
      aria-label="{bg.label} background"
      aria-pressed={currentBg === bg.id}
      title={bg.label}
      onclick={() => setBg(bg.id)}
    ></button>
  {/each}
</div>
