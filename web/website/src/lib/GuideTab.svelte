<script>
  let deviceTab = $state('server');
</script>

<style>
  .hero {
    text-align: center;
    padding: 2rem 1.5rem 1rem;
    max-width: 800px;
    margin: 0 auto;
  }

  .hero h2 {
    font-family: var(--font-heading);
    font-size: 3rem;
    font-weight: 800;
    line-height: 1.15;
    margin-bottom: 1.5rem;
    letter-spacing: -1px;
  }

  .hero p {
    color: var(--text-secondary);
    font-size: 1.15rem;
    margin-bottom: 2rem;
    max-width: 600px;
    margin-left: auto;
    margin-right: auto;
  }

  .device-tabs {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
    margin-bottom: 2rem;
  }

  .device-tab {
    background: var(--bg-card);
    border: 1px solid rgba(255, 255, 255, 0.05);
    padding: 1rem;
    border-radius: 12px;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    transition: background-color 0.3s ease, border-color 0.4s ease 0.1s, box-shadow 0.4s ease 0.1s;
  }

  .device-tab.active {
    background: var(--color-primary-glow);
    border-color: var(--color-primary);
    box-shadow: 0 4px 14px rgba(170, 59, 255, 0.15);
  }

  .device-tab-icon {
    font-size: 1.5rem;
  }

  .device-tab-title {
    text-align: left;
  }

  .device-tab-title h4 {
    font-family: var(--font-heading);
    font-size: 1rem;
    color: var(--text-primary);
  }

  .device-tab-title p {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .guide-step {
    margin-bottom: 2rem;
    background: rgba(255, 255, 255, 0.02);
    padding: 1.5rem;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.03);
  }

  .guide-step h4 {
    font-family: var(--font-heading);
    font-size: 1.15rem;
    color: var(--color-secondary);
    margin-bottom: 0.5rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .guide-step p {
    color: var(--text-secondary);
    font-size: 0.95rem;
    margin-bottom: 0.75rem;
  }

  pre {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 1rem;
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    color: HSL(190, 80%, 75%);
    margin-top: 0.75rem;
  }

  code {
    font-family: var(--font-mono);
    background: rgba(255, 255, 255, 0.05);
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    font-size: 0.85rem;
  }
</style>

<section class="hero" style="padding-top: 2rem;"> style="padding-top: 2rem;">
  <h2>Multi-Device Setup Guide</h2>
  <p>Compile, verify, and run the core marketplace infrastructure across all delivery surfaces.</p>
</section>

<!-- Device Tab Navigation -->
<div class="device-tabs">
  <button type="button" class="device-tab {deviceTab === 'server' ? 'active' : ''}" onclick={() => deviceTab = 'server'}>
    <span class="device-tab-icon">🖥️</span>
    <div class="device-tab-title">
      <h4>Marketplace Server</h4>
      <p>Rust API Core</p>
    </div>
  </button>
  <button type="button" class="device-tab {deviceTab === 'mcp' ? 'active' : ''}" onclick={() => deviceTab = 'mcp'}>
    <span class="device-tab-icon">🔌</span>
    <div class="device-tab-title">
      <h4>MCP Sidecar</h4>
      <p>Model Context Protocol</p>
    </div>
  </button>
  <button type="button" class="device-tab {deviceTab === 'mobile' ? 'active' : ''}" onclick={() => deviceTab = 'mobile'}>
    <span class="device-tab-icon">📱</span>
    <div class="device-tab-title">
      <h4>Mobile App</h4>
      <p>Tauri v2 + Svelte 5</p>
    </div>
  </button>
</div>

<!-- Guides Content -->
{#if deviceTab === 'server'}
  <div class="guide-step">
    <h4>1. Spin up PostgreSQL Database</h4>
    <p>Launch the database container using the local compose script:</p>
    <pre>docker compose -p marketplace -f compose.postgres.yml up -d</pre>
  </div>

  <div class="guide-step">
    <h4>2. Run Schema Migrations & Seed Data</h4>
    <p>Initialize the credit balances, negotiation rules, and seed sellers:</p>
    <pre>cargo run --bin bootstrap_schema</pre>
  </div>

  <div class="guide-step">
    <h4>3. Fire Up the Server</h4>
    <p>Binds to <code>127.0.0.1:3000</code> by default. Override using <code>MARKETPLACE_BIND</code> environment variable:</p>
    <pre>cargo run -p marketplace-server</pre>
  </div>

{:else if deviceTab === 'mcp'}
  <div class="guide-step">
    <h4>1. Build the MCP Executable</h4>
    <p>The Model Context Protocol sidecar connects desktop agents to the core server:</p>
    <pre>cargo build -p marketplace-mcp --release</pre>
  </div>

  <div class="guide-step">
    <h4>2. Configure Claude Desktop/Desktop Agent</h4>
    <p>Add the MCP tool configuration to your agent settings JSON:</p>
    <pre>{JSON.stringify({
  "mcpServers": {
    "marketplace": {
      "command": "./target/release/marketplace-mcp",
      "env": {
        "MARKETPLACE_API_KEY": "demo-secret-key",
        "MCP_TOOL_TIMEOUT_MS": "10000"
      }
    }
  }
}, null, 2)}</pre>
  </div>

  <div class="guide-step">
    <h4>3. Expose AI capabilities</h4>
    <p>The MCP server automatically exposes tools such as <code>search_listings</code>, <code>open_negotiation</code>, and <code>submit_offer</code> to the LLM agent.</p>
  </div>

{:else}
  <div class="guide-step">
    <h4>1. Install Mobile Dependencies</h4>
    <p>Tauri v2 + Svelte 5 runs the mobile clients. Navigate to the client workspace:</p>
    <pre>cd mobile/marketplace\nnpm install</pre>
  </div>

  <div class="guide-step">
    <h4>2. Run in Development Mode</h4>
    <p>Starts the frontend and compiles the Tauri native mobile runtime:</p>
    <pre>npm run tauri android dev  # For Android emulator\nnpm run tauri ios dev      # For iOS simulator</pre>
  </div>

  <div class="guide-step">
    <h4>3. Build Client Executables</h4>
    <p>Pack the final release packages for mobile platforms:</p>
    <pre>npm run tauri android build --release\nnpm run tauri ios build --release</pre>
  </div>
{/if}
