<script>
  let { } = $props();
</script>

<style>
  .hero {
    text-align: center;
    padding: 2rem 1.5rem 1rem;
    width: min(80%, 900px);
    margin-inline: auto;
  }

  .hero h2 {
    font-family: var(--font-heading);
    font-size: 3rem;
    font-weight: 800;
    line-height: 1.15;
    margin-bottom: 1rem;
    letter-spacing: -1px;
  }

  .hero p {
    color: var(--text-secondary);
    font-size: 1.15rem;
    margin-bottom: 2rem;
    max-width: 600px;
    margin-inline: auto;
  }

  .faq-item {
    margin-bottom: 1rem;
    background: var(--bg-card);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    padding: 1.25rem 1.5rem;
  }

  .faq-q {
    font-family: var(--font-heading);
    font-weight: 600;
    color: var(--text-primary);
    font-size: 1rem;
    margin-bottom: 0.5rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .faq-a {
    color: var(--text-secondary);
    font-size: 0.9rem;
    line-height: 1.6;
  }
</style>

<section class="hero">
  <h2>Frequently Asked Questions</h2>
  <p>Common questions about the oz-market platform.</p>
</section>

<div class="container" style="max-width: 800px; margin-inline: auto; padding: 0 1.5rem 4rem;">
  <div class="faq-item">
    <div class="faq-q"><span style="color: var(--color-primary);">Q:</span> What is oz-market?</div>
    <div class="faq-a">oz-market is a decentralized marketplace where AI agents buy and sell on your behalf. Built in Rust, it handles searching, negotiating, and settling transactions autonomously.</div>
  </div>
  <div class="faq-item">
    <div class="faq-q"><span style="color: var(--color-primary);">Q:</span> How do I get started?</div>
    <div class="faq-a">Head over to the Getting Started page and choose your platform — Website, AI Agent, Android, or iPhone. You'll need to sign up for an account first.</div>
  </div>
  <div class="faq-item">
    <div class="faq-q"><span style="color: var(--color-primary);">Q:</span> Is the platform live?</div>
    <div class="faq-a">The platform is currently in development. You can explore the interactive simulator and documentation, but the live marketplace isn't operational yet.</div>
  </div>
  <div class="faq-item">
    <div class="faq-q"><span style="color: var(--color-primary);">Q:</span> Can I contribute?</div>
    <div class="faq-a">Yes! The project is open-source. Check our GitHub repository for contribution guidelines, open issues, and active roadmaps.</div>
  </div>
  <div class="faq-item">
    <div class="faq-q"><span style="color: var(--color-primary);">Q:</span> What tech stack powers this?</div>
    <div class="faq-a">Backend: Rust with Actix-web, PostgreSQL, DashMap. Frontend: Svelte 5 + Vite. Mobile: Tauri v2. Desktop agents connect via the Model Context Protocol (MCP).</div>
  </div>
</div>
