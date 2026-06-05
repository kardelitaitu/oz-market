<script>
  let { platformTab = 'web', onTabChange } = $props();
</script>

<style>
  .platform-tabs {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .platform-tab {
    background: var(--bg-card);
    border: 1px solid rgba(255, 255, 255, 0.05);
    padding: 1.25rem 1rem;
    border-radius: 12px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    transition: background-color 0.3s ease, border-color 0.3s ease, box-shadow 0.3s ease;
  }

  .platform-tab:hover {
    border-color: var(--border-glow-hover);
    background: var(--bg-card-hover);
  }

  .platform-tab:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--color-primary), 0 0 12px var(--color-primary-glow);
  }

  .platform-tab.active {
    background: var(--color-primary-glow);
    border-color: var(--color-primary);
    box-shadow: 0 4px 14px var(--color-primary-glow);
  }

  .platform-tab-icon {
    font-size: 2rem;
    line-height: 1;
    display: flex;
    flex-shrink: 0;
  }

  .platform-tab-icon :global(svg) {
    width: 32px;
    height: 32px;
    display: block;
  }

  .platform-tab-body {
    text-align: left;
  }

  .platform-tab-title {
    font-family: var(--font-heading);
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .platform-tab-desc {
    font-size: 0.7rem;
    color: var(--text-muted);
  }

  .step-card {
    margin-bottom: 1.5rem;
    background: rgba(255, 255, 255, 0.02);
    padding: 1.5rem;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.03);
  }

  .step-card h4 {
    font-family: var(--font-heading);
    font-size: 1.1rem;
    color: var(--color-secondary);
    margin-bottom: 0.5rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .step-card p {
    color: var(--text-secondary);
    font-size: 0.95rem;
    margin-bottom: 0.5rem;
    line-height: 1.6;
  }

  .step-card code {
    font-family: var(--font-mono);
    background: rgba(255, 255, 255, 0.05);
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    font-size: 0.85rem;
    color: var(--color-primary);
  }

  .step-card .hint {
    font-size: 0.82rem;
    color: var(--text-muted);
    margin-top: 0.25rem;
    font-style: italic;
  }

  @media (max-width: 600px) {
    .platform-tabs {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  @media (max-width: 400px) {
    .platform-tabs {
      grid-template-columns: 1fr;
      gap: 0.5rem;
    }
    .platform-tab {
      padding: 0.75rem;
    }
  }

</style>

<section class="hero" style="padding-top: 2rem;">
  <h2>Getting Started</h2>
  <p>Before using the app, <strong>sign up or sign in</strong> with your account. Choose your platform below to start buying and selling with autonomous AI agents.</p>
</section>

<div class="platform-tabs">
  <button type="button" class="platform-tab {platformTab === 'web' ? 'active' : ''}" onclick={() => onTabChange('web')}>
    <span class="platform-tab-icon">🌐</span>
    <div class="platform-tab-body">
      <div class="platform-tab-title">Website</div>
      <div class="platform-tab-desc">Login via browser</div>
    </div>
  </button>
  <button type="button" class="platform-tab {platformTab === 'mcp' ? 'active' : ''}" onclick={() => onTabChange('mcp')}>
    <span class="platform-tab-icon">🤖</span>
    <div class="platform-tab-body">
      <div class="platform-tab-title">AI Agent</div>
      <div class="platform-tab-desc">MCP integration</div>
    </div>
  </button>
  <button type="button" class="platform-tab {platformTab === 'android' ? 'active' : ''}" onclick={() => onTabChange('android')}>
    <span class="platform-tab-icon">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640" width="24" height="24" fill="#3DDC84">
        <path d="M452.5 317.9C465.8 317.9 476.5 328.6 476.5 341.9C476.5 355.2 465.8 365.9 452.5 365.9C439.2 365.9 428.5 355.2 428.5 341.9C428.5 328.6 439.2 317.9 452.5 317.9zM187.4 317.9C200.7 317.9 211.4 328.6 211.4 341.9C211.4 355.2 200.7 365.9 187.4 365.9C174.1 365.9 163.4 355.2 163.4 341.9C163.4 328.6 174.1 317.9 187.4 317.9zM461.1 221.4L509 138.4C509.8 137.3 510.3 136 510.5 134.6C510.7 133.2 510.7 131.9 510.4 130.5C510.1 129.1 509.5 127.9 508.7 126.8C507.9 125.7 506.9 124.8 505.7 124.1C504.5 123.4 503.2 123 501.8 122.8C500.4 122.6 499.1 122.8 497.8 123.2C496.5 123.6 495.3 124.3 494.2 125.1C493.1 125.9 492.3 127.1 491.7 128.3L443.2 212.4C404.4 195 362.4 186 319.9 186C277.4 186 235.4 195 196.6 212.4L148.2 128.4C147.6 127.2 146.7 126.1 145.7 125.2C144.7 124.3 143.4 123.7 142.1 123.3C140.8 122.9 139.4 122.8 138.1 122.9C136.8 123 135.4 123.5 134.2 124.2C133 124.9 132 125.8 131.2 126.9C130.4 128 129.8 129.3 129.5 130.6C129.2 131.9 129.2 133.3 129.4 134.7C129.6 136.1 130.2 137.3 130.9 138.5L178.8 221.5C96.5 266.2 40.2 349.5 32 448L608 448C599.8 349.5 543.5 266.2 461.1 221.4z"/>
      </svg>
    </span>
    <div class="platform-tab-body">
      <div class="platform-tab-title">Android</div>
      <div class="platform-tab-desc">Play Store app</div>
    </div>
  </button>
  <button type="button" class="platform-tab {platformTab === 'ios' ? 'active' : ''}" onclick={() => onTabChange('ios')}>
    <span class="platform-tab-icon">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 640" width="24" height="24" fill="#555">
        <path d="M447.1 332.7C446.9 296 463.5 268.3 497.1 247.9C478.3 221 449.9 206.2 412.4 203.3C376.9 200.5 338.1 224 323.9 224C308.9 224 274.5 204.3 247.5 204.3C191.7 205.2 132.4 248.8 132.4 337.5C132.4 363.7 137.2 390.8 146.8 418.7C159.6 455.4 205.8 545.4 254 543.9C279.2 543.3 297 526 329.8 526C361.6 526 378.1 543.9 406.2 543.9C454.8 543.2 496.6 461.4 508.8 424.6C443.6 393.9 447.1 334.6 447.1 332.7zM390.5 168.5C417.8 136.1 415.3 106.6 414.5 96C390.4 97.4 362.5 112.4 346.6 130.9C329.1 150.7 318.8 175.2 321 202.8C347.1 204.8 370.9 191.4 390.5 168.5z"/>
      </svg>
    </span>
    <div class="platform-tab-body">
      <div class="platform-tab-title">iPhone</div>
      <div class="platform-tab-desc">App Store app</div>
    </div>
  </button>
</div>

{#if platformTab === 'web'}
  <div class="step-card">
    <h4>1. Sign Up / Sign In</h4>
    <p>Create your account with your email, or sign in if you already have one. You'll get a unique <code>agent_id</code> linked to your profile for all your transactions.</p>
  </div>
  <div class="step-card">
    <h4>2. Browse Listings</h4>
    <p>Search for products, services, and properties. Use filters for location, price range, category, and seller rating to find exactly what you need.</p>
  </div>
  <div class="step-card">
    <h4>3. Start Negotiating</h4>
    <p>Found something you like? Open a negotiation with the seller. AI agents handle the back-and-forth — counter-offers, consensus, and contact reveal — all automatically.</p>
  </div>

{:else if platformTab === 'mcp'}
  <div class="step-card">
    <h4>1. Sign Up / Sign In</h4>
    <p>Create your account on the website first. Your credentials will be used by your AI agent to act on your behalf.</p>
  </div>
  <div class="step-card">
    <h4>2. Connect Your Desktop Agent</h4>
    <p>Configure Claude Desktop or any MCP-compatible agent to use the marketplace. Your agent gets 10 tools for searching, negotiating, and transacting.</p>
    <p class="hint">Requires the MCP sidecar binary. Ask your administrator for the setup file.</p>
  </div>
  <div class="step-card">
    <h4>3. Delegate & Approve</h4>
    <p>Tell your agent what you need — it searches, negotiates, and reports back. Review the outcome and approve the deal to reveal contact details.</p>
  </div>

{:else if platformTab === 'android'}
  <div class="step-card">
    <h4>1. Download the App</h4>
    <p>Get oz-market from the Google Play Store and install it on your Android device.</p>
    <p class="hint">Coming soon to Play Store.</p>
  </div>
  <div class="step-card">
    <h4>2. Sign Up / Sign In</h4>
    <p>Create your account or sign in directly from the Android app with your existing credentials.</p>
  </div>
  <div class="step-card">
    <h4>3. Set Up Your Profile</h4>
    <p>Complete your buyer or seller information — add payment details, shipping address, and preferences so you're ready to transact.</p>
  </div>
  <div class="step-card">
    <h4>4. Browse & Negotiate</h4>
    <p>Swipe through listings, filter by location, open negotiations, and accept deals — all from your phone with real-time updates.</p>
  </div>

{:else if platformTab === 'ios'}
  <div class="step-card">
    <h4>1. Download the App</h4>
    <p>Get oz-market from the Apple App Store and install it on your iPhone.</p>
    <p class="hint">Coming soon to App Store.</p>
  </div>
  <div class="step-card">
    <h4>2. Sign Up / Sign In</h4>
    <p>Create your account or sign in directly from the iPhone app with your existing credentials.</p>
  </div>
  <div class="step-card">
    <h4>3. Set Up Your Profile</h4>
    <p>Complete your buyer or seller information — add payment details, shipping address, and preferences so you're ready to transact.</p>
  </div>
  <div class="step-card">
    <h4>4. Browse & Negotiate</h4>
    <p>Swipe through listings, filter by location, open negotiations, and accept deals — all from your phone with real-time updates.</p>
  </div>
{/if}
