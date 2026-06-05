<script>
  /**
   * @typedef {'idle'|'listing'|'negotiating'|'consensus'|'revealing'|'completed'} SimState
   * @type {{ simState: SimState }}
   */
  let { simState = 'idle' } = $props();
</script>

<style>
  .flow-diagram {
    width: 100%;
    overflow: visible;
    margin-bottom: 1rem;
    min-height: 100px;
  }

  .flow-node-circle {
    fill: rgba(0, 0, 0, 0.45);
    stroke: var(--color-primary);
    stroke-width: 1.5;
    transition: stroke 0.4s ease;
  }

  .flow-node-label {
    fill: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 500;
    text-anchor: middle;
  }

  .flow-path {
    fill: none;
    stroke: rgba(255, 255, 255, 0.08);
    stroke-width: 1.5;
    stroke-dasharray: 6 4;
    transition: stroke 0.4s ease;
  }

  .flow-path.active {
    stroke: var(--color-primary);
    opacity: 0.8;
  }

  .flow-laser {
    r: 3;
    fill: var(--color-secondary);
    opacity: 0;
  }

  .flow-laser.active {
    opacity: 1;
    animation: laserTravel 1.6s ease-in-out infinite;
  }

  @keyframes laserTravel {
    0%   { offset-distance: 0%;   opacity: 0; }
    10%  { opacity: 1; }
    90%  { opacity: 1; }
    100% { offset-distance: 100%; opacity: 0; }
  }

  @keyframes nodePulse {
    0%, 100% { r: 20; opacity: 1; }
    50%       { r: 23; opacity: 0.75; }
  }

  .flow-node-active .flow-node-circle {
    stroke: var(--color-secondary);
    animation: nodePulse 1.2s ease-in-out infinite;
  }
</style>

<svg
  class="flow-diagram"
  viewBox="0 0 320 80"
  xmlns="http://www.w3.org/2000/svg"
  aria-label="Agent negotiation flow diagram"
>
  <defs>
    <filter id="node-glow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="3" result="blur"/>
      <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
  </defs>

  <!-- ─── Connector lines ─── -->
  <line
    class="flow-path {simState !== 'idle' ? 'active' : ''}"
    x1="68" y1="40" x2="134" y2="40"
  />
  <line
    class="flow-path {simState === 'listing' || simState === 'negotiating' || simState === 'consensus' || simState === 'revealing' || simState === 'completed' ? 'active' : ''}"
    x1="186" y1="40" x2="256" y2="40"
  />

  <!-- ─── Signal dots: inline path avoids mpath ID lookup failures ─── -->

  <!-- Buyer → Server forward dot -->
  {#if simState !== 'idle'}
    <circle r="3.5" fill="var(--color-secondary)" opacity="0.9">
      <animateMotion
        dur="1.1s"
        repeatCount="indefinite"
        calcMode="linear"
        path="M 68,40 L 134,40"
      />
    </circle>
  {/if}

  <!-- Server → Buyer return dot (counter-offer) -->
  {#if simState === 'negotiating'}
    <circle r="2.8" fill="var(--color-accent)" opacity="0.75">
      <animateMotion
        dur="1.1s"
        repeatCount="indefinite"
        calcMode="linear"
        path="M 134,40 L 68,40"
        begin="0.55s"
      />
    </circle>
  {/if}

  <!-- Server → Seller forward dot -->
  {#if simState === 'listing' || simState === 'negotiating' || simState === 'consensus' || simState === 'revealing' || simState === 'completed'}
    <circle r="3.5" fill="var(--color-secondary)" opacity="0.9">
      <animateMotion
        dur="1.1s"
        repeatCount="indefinite"
        calcMode="linear"
        path="M 186,40 L 256,40"
      />
    </circle>
  {/if}

  <!-- Seller → Server return dot (during negotiation) -->
  {#if simState === 'negotiating'}
    <circle r="2.8" fill="var(--color-accent)" opacity="0.75">
      <animateMotion
        dur="1.1s"
        repeatCount="indefinite"
        calcMode="linear"
        path="M 256,40 L 186,40"
        begin="0.55s"
      />
    </circle>
  {/if}

  <!-- ─── BUYER node (cx=40) ─── -->
  <g filter="url(#node-glow)">
    <circle
      class="flow-node-circle"
      cx="40" cy="40" r="24"
      style="stroke: {simState === 'negotiating' || simState === 'revealing' ? 'var(--color-secondary)' : 'var(--color-primary)'}; stroke-width: {simState === 'negotiating' || simState === 'revealing' ? '2' : '1.5'};"
    />
    <circle cx="40" cy="31" r="5" fill="var(--color-primary)" opacity="0.9"/>
    <path d="M 33 43 Q 40 38 47 43 L 47 52 L 33 52 Z" fill="var(--color-primary)" opacity="0.7"/>
  </g>
  <text class="flow-node-label" x="40" y="73">BUYER</text>

  <!-- ─── SERVER node (cx=160) ─── -->
  <g filter="url(#node-glow)">
    <circle
      class="flow-node-circle"
      cx="160" cy="40" r="26"
      style="stroke: {simState !== 'idle' ? 'var(--color-secondary)' : 'var(--color-primary)'}; stroke-width: {simState !== 'idle' ? '2' : '1.5'};"
    />
    <circle cx="160" cy="40" r="9" fill="none" stroke="var(--color-secondary)" stroke-width="1.5" opacity="0.8"/>
    <circle cx="160" cy="40" r="4" fill="var(--color-secondary)" opacity="0.8"/>
    {#each [0,45,90,135,180,225,270,315] as deg}
      <rect
        x="158.5" y="27"
        width="3" height="5"
        fill="var(--color-secondary)"
        opacity="0.75"
        transform="rotate({deg} 160 40)"
      />
    {/each}
  </g>
  <text class="flow-node-label" x="160" y="75">SERVER</text>

  <!-- ─── SELLER node (cx=280) ─── -->
  <g filter="url(#node-glow)">
    <circle
      class="flow-node-circle"
      cx="280" cy="40" r="24"
      style="stroke: {simState === 'listing' || simState === 'negotiating' || simState === 'consensus' ? 'var(--color-secondary)' : 'var(--color-primary)'}; stroke-width: {simState === 'listing' || simState === 'negotiating' || simState === 'consensus' ? '2' : '1.5'};"
    />
    <circle cx="280" cy="31" r="5" fill="var(--color-secondary)" opacity="0.9"/>
    <path d="M 273 43 Q 280 38 287 43 L 287 52 L 273 52 Z" fill="var(--color-secondary)" opacity="0.7"/>
  </g>
  <text class="flow-node-label" x="280" y="73">SELLER</text>

  <!-- ─── Consensus ring ─── -->
  {#if simState === 'consensus' || simState === 'revealing' || simState === 'completed'}
    <circle cx="160" cy="40" r="32" fill="none"
      stroke="var(--color-success)" stroke-width="1.5"
      stroke-dasharray="5 3" opacity="0.6"
    >
      <animateTransform attributeName="transform" type="rotate"
        from="0 160 40" to="360 160 40" dur="3s" repeatCount="indefinite"/>
    </circle>
  {/if}
</svg>
