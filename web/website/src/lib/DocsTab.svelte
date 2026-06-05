<style>
  .hero {
    max-width: 800px;
  }

  .hero h2 {
    margin-bottom: 1.5rem;
  }

  .docs-list {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 1rem;
  }

  .doc-item {
    background: var(--bg-card);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    padding: 1.25rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    transition: background-color 0.3s ease, border-color 0.4s ease 0.1s;
  }

  .doc-item:hover {
    border-color: var(--color-primary);
    background: var(--bg-card-hover);
  }

  .doc-info {
    display: flex;
    flex-direction: column;
  }

  .doc-title {
    font-weight: 600;
    color: var(--text-primary);
  }

  .doc-meta {
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-top: 0.25rem;
  }

  .btn-arrow {
    font-size: 1.25rem;
    color: var(--text-muted);
  }

  .doc-item:hover .btn-arrow {
    color: var(--color-secondary);
    transform: translateX(3px);
  }
</style>

<section class="hero" style="padding-top: 2rem;">
  <h2>Documentation Hub</h2>
  <p>Everything you need — from end-user guides to developer architecture docs.</p>
</section>

<div class="card" style="margin-bottom: 2rem;">
  <h3 style="color: var(--color-secondary);">👤 Docs for End User</h3>
  <p>Guides and references for using the oz-market platform.</p>

  <div class="docs-list">
    <a href="docs/GOALS.md" class="doc-item">
      <div class="doc-info">
        <span class="doc-title">Marketplace Overview & Goals</span>
        <span class="doc-meta">docs/GOALS.md</span>
      </div>
      <span class="btn-arrow">→</span>
    </a>
    <a href="docs/CLI-README.md" class="doc-item">
      <div class="doc-info">
        <span class="doc-title">CLI Usage Guide</span>
        <span class="doc-meta">docs/CLI-README.md</span>
      </div>
      <span class="btn-arrow">→</span>
    </a>
    <a href="docs/deploy.md" class="doc-item">
      <div class="doc-info">
        <span class="doc-title">Deployment Guide</span>
        <span class="doc-meta">docs/deploy.md</span>
      </div>
      <span class="btn-arrow">→</span>
    </a>
    <a href="docs/TESTING.md" class="doc-item">
      <div class="doc-info">
        <span class="doc-title">Testing Guide</span>
        <span class="doc-meta">docs/TESTING.md</span>
      </div>
      <span class="btn-arrow">→</span>
    </a>
  </div>
</div>

<div class="card" style="margin-bottom: 2rem;">
  <h3 style="color: var(--color-primary);">⚙️ Docs for Developer</h3>
  <p>Architecture, API contracts, and system design specifications.</p>

  <div class="docs-list">
    <a href="docs/01-whitepaper/README.md" class="doc-item">
      <div class="doc-info">
        <span class="doc-title">Project Whitepaper Overview</span>
        <span class="doc-meta">docs/01-whitepaper/README.md</span>
      </div>
      <span class="btn-arrow">→</span>
    </a>
    <a href="docs/01-whitepaper/10-api-contract.md" class="doc-item">
      <div class="doc-info">
        <span class="doc-title">Frozen V1 API Contract</span>
        <span class="doc-meta">docs/01-whitepaper/10-api-contract.md</span>
      </div>
      <span class="btn-arrow">→</span>
    </a>
    <a href="docs/01-whitepaper/11-identity-authz.md" class="doc-item">
      <div class="doc-info">
        <span class="doc-title">Identity, Claims & Authz Matrix</span>
        <span class="doc-meta">docs/01-whitepaper/11-identity-authz.md</span>
      </div>
      <span class="btn-arrow">→</span>
    </a>
    <a href="docs/server/module-layout.md" class="doc-item">
      <div class="doc-info">
        <span class="doc-title">Server Crate Architecture</span>
        <span class="doc-meta">docs/server/module-layout.md</span>
      </div>
      <span class="btn-arrow">→</span>
    </a>
  </div>
</div>

<div class="card">
  <h3 style="color: var(--color-secondary);">🚀 Active Roadmaps (Specifications)</h3>
  <p>Upcoming infrastructure features governing the system scaling phases.</p>

  <div class="docs-list">
    <a href="docs/specs/_active/0024-distributed-ledger-cache-redis/README.md" class="doc-item">
      <div class="doc-info">
        <span class="doc-title">Spec 0024: Redis Distributed Cache</span>
        <span class="doc-meta">Clustered transactions & pub/sub eviction</span>
      </div>
      <span class="btn-arrow">→</span>
    </a>
    <a href="docs/specs/_active/0025-zero-copy-ffi-serialization/README.md" class="doc-item">
      <div class="doc-info">
        <span class="doc-title">Spec 0025: MessagePack FFI</span>
        <span class="doc-meta">Zero-copy client-side FFI optimizations</span>
      </div>
      <span class="btn-arrow">→</span>
    </a>
    <a href="docs/specs/_active/0026-transactional-outbox-pattern/README.md" class="doc-item">
      <div class="doc-info">
        <span class="doc-title">Spec 0026: Transactional Outbox</span>
        <span class="doc-meta">Guaranteed at-least-once event delivery</span>
      </div>
      <span class="btn-arrow">→</span>
    </a>
    <a href="docs/specs/_active/0027-refresh-token-rotation-jwt-blacklist/README.md" class="doc-item">
      <div class="doc-info">
        <span class="doc-title">Spec 0027: Token Rotation & JWT Blacklist</span>
        <span class="doc-meta">Security-focused session breach detection</span>
      </div>
      <span class="btn-arrow">→</span>
    </a>
  </div>
</div>
