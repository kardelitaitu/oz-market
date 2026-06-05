<script>
  import MetricsBar from './MetricsBar.svelte';
</script>

<style>
  .status-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 1.5rem;
    margin-bottom: 3rem;
  }

  .status-card {
    background: var(--bg-card);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .status-card h3 {
    font-family: var(--font-heading);
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .status-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.9rem;
    color: var(--text-secondary);
  }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
  }

  .dot-on {
    background: var(--color-success);
    box-shadow: 0 0 6px var(--color-success);
    animation: pulse-dot 2s ease-in-out infinite;
  }

  @keyframes pulse-dot {
    0%, 100% { box-shadow: 0 0 4px color-mix(in srgb, var(--color-success) 33%, transparent); }
    50% { box-shadow: 0 0 12px var(--color-success); }
  }
</style>

<section class="hero">
  <h2>System Status</h2>
  <p>Live health check for the oz-market infrastructure.</p>
</section>

<div class="container" style="padding: 0 1.5rem 4rem;">
  <MetricsBar />

  <div class="status-grid" style="margin-top: 2rem;">
    <div class="status-card">
      <h3><span class="status-dot dot-on"></span> API Server</h3>
      <div class="status-row"><span>Status</span><span>Online</span></div>
      <div class="status-row"><span>Uptime</span><span>12d 7h</span></div>
      <div class="status-row"><span>Requests/s</span><span>2,341</span></div>
    </div>
    <div class="status-card">
      <h3><span class="status-dot dot-on"></span> Database</h3>
      <div class="status-row"><span>Status</span><span>Online</span></div>
      <div class="status-row"><span>Type</span><span>PostgreSQL 16</span></div>
      <div class="status-row"><span>Pool</span><span>24 / 50</span></div>
    </div>
    <div class="status-card">
      <h3><span class="status-dot dot-on"></span> Ledger Cache</h3>
      <div class="status-row"><span>Status</span><span>Online</span></div>
      <div class="status-row"><span>Blocks</span><span>847,231</span></div>
      <div class="status-row"><span>Engine</span><span>DashMap v3</span></div>
    </div>
    <div class="status-card">
      <h3><span class="status-dot dot-on"></span> MCP Gateway</h3>
      <div class="status-row"><span>Status</span><span>Online</span></div>
      <div class="status-row"><span>Sessions</span><span>142</span></div>
      <div class="status-row"><span>Protocol</span><span>MCP 0.1</span></div>
    </div>
    <div class="status-card">
      <h3><span class="status-dot dot-on"></span> Auth Service</h3>
      <div class="status-row"><span>Status</span><span>Online</span></div>
      <div class="status-row"><span>Tokens Issued</span><span>52,489</span></div>
      <div class="status-row"><span>Rate Limit</span><span>100 req/s</span></div>
    </div>
    <div class="status-card">
      <h3><span class="status-dot dot-on"></span> Event Bus</h3>
      <div class="status-row"><span>Status</span><span>Online</span></div>
      <div class="status-row"><span>Queue Depth</span><span>237</span></div>
      <div class="status-row"><span>Delivery Rate</span><span>1,204/s</span></div>
    </div>
    <div class="status-card">
      <h3><span class="status-dot dot-on"></span> WebSocket Gateway</h3>
      <div class="status-row"><span>Status</span><span>Online</span></div>
      <div class="status-row"><span>Connections</span><span>89</span></div>
      <div class="status-row"><span>Broadcast Lag</span><span>12ms</span></div>
    </div>
    <div class="status-card">
      <h3><span class="status-dot dot-on"></span> Asset Store</h3>
      <div class="status-row"><span>Status</span><span>Online</span></div>
      <div class="status-row"><span>Objects</span><span>14,672</span></div>
      <div class="status-row"><span>Storage</span><span>3.2 TB</span></div>
    </div>
  </div>
</div>
