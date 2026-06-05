<script>
  import { catalog } from './simulator.svelte.js';

  // Props
  let { block, onclose } = $props();

  // Find item details reactively
  let itemInfo = $derived(catalog.find(c => c.name === block.item));
  let basePrice = $derived(itemInfo ? itemInfo.basePrice : Math.round(block.price * 1.35));
  let discountAmount = $derived(basePrice - block.price);
  let discountPct = $derived(basePrice > 0 ? (discountAmount / basePrice) * 100 : 0);

  // Reconstruct the bid sequence reactively
  let lowball = $derived(Math.round(basePrice * 0.29));
  let c1 = $derived(Math.round(basePrice * 0.93));
  let c2 = $derived(Math.round(basePrice * 0.50));
  let c3 = $derived(Math.round(basePrice * 0.86));
  let c4 = $derived(Math.round(basePrice * 0.57));
  let deal = $derived(block.price);

  // Reconstructed transcript steps reactively
  let transcriptSteps = $derived([
    { speaker: 'seller', msg: `Publishing new product listing: "${block.item}" at base price $${basePrice}.00.` },
    { speaker: 'buyer',  msg: `Discovered active listing via search. Initiating negotiation...` },
    { speaker: 'buyer',  msg: `Sent initial low-ball offer: $${lowball}.00.` },
    { speaker: 'seller', msg: `Counter-offer received: $${c1}.00 (min_seller_rating check: PASS).` },
    { speaker: 'buyer',  msg: `Countering with price history average: $${c2}.00.` },
    { speaker: 'seller', msg: `Adjusting bid within discount limits. Counter-offer: $${c3}.00.` },
    { speaker: 'buyer',  msg: `Near upper utility limit. Final offer: $${c4}.00.` },
    { speaker: 'seller', msg: `Final counter split difference: $${deal}.00.` },
    { speaker: 'buyer',  msg: `Accept offer $${deal}.00. Consensus reached! Writing to ledger cache...` }
  ]);

  // Cryptographic Claims (deterministic based on block hash)
  let txSig = $derived(`sig-tx-${block.hash.substring(2, 10)}`);
  let buyerPK = $derived(`0x3aa7f60${block.hash.substring(4, 9)}ff3b107c1b489a2b9`);
  let sellerPK = $derived(`0xf81c92d${block.hash.substring(2, 7)}ee4209fa8b122e4c0`);
  let consensusRoot = $derived(`0x2e01df3${block.hash.substring(2, 8)}d85a1e74f32a76f2e`);

  // UI state
  let copiedField = $state('');

  function copyToClipboard(text, field) {
    if (typeof navigator !== 'undefined' && navigator.clipboard) {
      navigator.clipboard.writeText(text);
      copiedField = field;
      setTimeout(() => {
        copiedField = '';
      }, 1500);
    }
  }

  function handleOverlayClick(e) {
    if (e.target === e.currentTarget) {
      onclose();
    }
  }

  function handleKeydown(e) {
    if (e.key === 'Escape') {
      onclose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Modal Overlay -->
<div 
  class="modal-overlay" 
  onclick={handleOverlayClick} 
  onkeydown={e => { if (e.key === 'Enter' || e.key === ' ') { onclose(); e.preventDefault(); } }}
  role="button" 
  tabindex="0"
  aria-label="Close modal overlay"
>
  <!-- Modal Content Container -->
  <div class="modal-card" role="document" tabindex="-1">
    <!-- Close Button -->
    <button class="modal-close-btn" onclick={onclose} aria-label="Close modal">×</button>

    <!-- Header -->
    <div class="modal-header">
      <h4 class="modal-title">🔍 Transaction Block Inspector</h4>
      <div class="modal-hash">{block.hash}</div>
    </div>

    <!-- Scrollable container -->
    <div class="modal-body">
      <!-- Details Grid -->
      <div class="details-grid">
        <div class="detail-item">
          <span class="detail-label">Product Name</span>
          <span class="detail-value" style="color: var(--text-primary);">{block.item}</span>
        </div>
        <div class="detail-item">
          <span class="detail-label">Transacted Price</span>
          <span class="detail-value" style="color: var(--color-accent); font-weight: 700;">${block.price}.00</span>
        </div>
        <div class="detail-item">
          <span class="detail-label">Base Price</span>
          <span class="detail-value">${basePrice}.00</span>
        </div>
        <div class="detail-item">
          <span class="detail-label">Total Discount</span>
          <span class="detail-value" style="color: var(--color-success); font-weight: 600;">
            -${discountAmount}.00 ({discountPct.toFixed(1)}%)
          </span>
        </div>
      </div>

      <!-- Cryptographic Proofs -->
      <div class="section-title">🔒 Cryptographic Proofs & Claims</div>
      <div class="crypto-card">
        <!-- TX Signature -->
        <div class="crypto-field">
          <div>
            <span class="crypto-label">Transaction Signature</span>
            <span class="crypto-value">{txSig}</span>
          </div>
          <button class="copy-btn" onclick={() => copyToClipboard(txSig, 'sig')}>
            {copiedField === 'sig' ? '✓ Copied' : '📋 Copy'}
          </button>
        </div>

        <!-- Buyer Public Key -->
        <div class="crypto-field">
          <div>
            <span class="crypto-label">Buyer Public Address (PK)</span>
            <span class="crypto-value">{buyerPK}</span>
          </div>
          <button class="copy-btn" onclick={() => copyToClipboard(buyerPK, 'buyer')}>
            {copiedField === 'buyer' ? '✓ Copied' : '📋 Copy'}
          </button>
        </div>

        <!-- Seller Public Key -->
        <div class="crypto-field">
          <div>
            <span class="crypto-label">Seller Public Address (PK)</span>
            <span class="crypto-value">{sellerPK}</span>
          </div>
          <button class="copy-btn" onclick={() => copyToClipboard(sellerPK, 'seller')}>
            {copiedField === 'seller' ? '✓ Copied' : '📋 Copy'}
          </button>
        </div>

        <!-- Consensus Hash -->
        <div class="crypto-field" style="border-bottom: none; padding-bottom: 0; margin-bottom: 0;">
          <div>
            <span class="crypto-label">Consensus State Root Hash</span>
            <span class="crypto-value">{consensusRoot}</span>
          </div>
          <button class="copy-btn" onclick={() => copyToClipboard(consensusRoot, 'root')}>
            {copiedField === 'root' ? '✓ Copied' : '📋 Copy'}
          </button>
        </div>
      </div>

      <!-- Dialogue Reconstruction -->
      <div class="section-title">💬 Reconstructed Negotiation Dialogue</div>
      <div class="dialogue-list">
        {#each transcriptSteps as step}
          <div class="dialogue-bubble {step.speaker}">
            <span class="dialogue-speaker">
              {step.speaker === 'buyer' ? 'Buyer Agent' : 'Seller Agent'}
            </span>
            <p class="dialogue-msg">{step.msg}</p>
          </div>
        {/each}
      </div>
    </div>

    <!-- Footer -->
    <div class="modal-footer">
      <span style="color: var(--text-muted); font-size: 0.65rem;">
        Timestamp: {block.ts} | Block Status: Committed & Verified
      </span>
      <button class="action-btn" onclick={onclose}>Done</button>
    </div>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    z-index: 1000;
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 1.5rem;
  }

  .modal-card {
    background: var(--bg-card);
    border: 1px solid var(--border-glow);
    border-radius: 16px;
    box-shadow: 0 20px 50px rgba(0,0,0,0.6);
    width: 100%;
    max-width: 550px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    position: relative;
    overflow: hidden;
    animation: zoomIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes zoomIn {
    from { opacity: 0; transform: scale(0.95); }
    to { opacity: 1; transform: scale(1); }
  }

  .modal-close-btn {
    position: absolute;
    top: 1rem;
    right: 1.2rem;
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 1.8rem;
    cursor: pointer;
    line-height: 1;
    z-index: 10;
    transition: color 0.2s;
  }

  .modal-close-btn:hover {
    color: var(--text-primary);
  }

  .modal-header {
    padding: 1.5rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .modal-title {
    font-family: var(--font-heading);
    font-size: 1.15rem;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0;
  }

  .modal-hash {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--color-secondary);
    margin-top: 0.35rem;
  }

  .modal-body {
    padding: 1.5rem;
    overflow-y: auto;
    flex-grow: 1;
    scrollbar-width: thin;
    scrollbar-color: var(--border-glow) transparent;
  }

  .details-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .detail-item {
    display: flex;
    flex-direction: column;
    background: rgba(255, 255, 255, 0.01);
    border: 1px solid rgba(255, 255, 255, 0.03);
    border-radius: 8px;
    padding: 0.6rem 0.8rem;
  }

  .detail-label {
    font-size: 0.65rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 0.2rem;
  }

  .detail-value {
    font-size: 0.88rem;
    color: var(--text-secondary);
    font-family: var(--font-sans);
  }

  .section-title {
    font-family: var(--font-heading);
    font-size: 0.8rem;
    font-weight: 600;
    text-transform: uppercase;
    color: var(--text-muted);
    letter-spacing: 1px;
    margin-top: 1.5rem;
    margin-bottom: 0.6rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
    padding-bottom: 0.25rem;
  }

  .crypto-card {
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--border-glow);
    border-radius: 10px;
    padding: 0.8rem 1rem;
  }

  .crypto-field {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    padding-bottom: 0.5rem;
    margin-bottom: 0.5rem;
    gap: 1rem;
  }

  .crypto-label {
    display: block;
    font-size: 0.62rem;
    color: var(--text-muted);
    margin-bottom: 0.1rem;
  }

  .crypto-value {
    display: block;
    font-family: var(--font-mono);
    font-size: 0.68rem;
    color: var(--text-secondary);
    word-break: break-all;
  }

  .copy-btn {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--text-muted);
    font-size: 0.65rem;
    padding: 0.2rem 0.45rem;
    border-radius: 4px;
    cursor: pointer;
    min-width: 60px;
    text-align: center;
    transition: background-color 0.2s, color 0.2s;
  }

  .copy-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-primary);
  }

  .dialogue-list {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    margin-top: 0.5rem;
  }

  .dialogue-bubble {
    padding: 0.6rem 0.8rem;
    border-radius: 8px;
    max-width: 90%;
    font-family: var(--font-sans);
    line-height: 1.4;
  }

  .dialogue-bubble.buyer {
    background: rgba(var(--color-primary-rgb, 0, 242, 254), 0.05);
    border-left: 3px solid var(--color-primary);
    align-self: flex-start;
  }

  .dialogue-bubble.seller {
    background: rgba(var(--color-secondary-rgb, 217, 70, 239), 0.05);
    border-left: 3px solid var(--color-secondary);
    align-self: flex-end;
  }

  .dialogue-speaker {
    display: block;
    font-size: 0.62rem;
    font-weight: 700;
    margin-bottom: 0.15rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .buyer .dialogue-speaker {
    color: var(--color-primary);
  }

  .seller .dialogue-speaker {
    color: var(--color-secondary);
  }

  .dialogue-msg {
    font-size: 0.76rem;
    color: var(--text-secondary);
    margin: 0;
  }

  .modal-footer {
    padding: 1rem 1.5rem;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: rgba(0, 0, 0, 0.1);
  }

  .action-btn {
    background: var(--color-primary-glow);
    border: 1px solid var(--color-primary);
    color: var(--color-primary);
    font-weight: 600;
    font-size: 0.75rem;
    padding: 0.35rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    transition: background-color 0.2s, color 0.2s;
  }

  .action-btn:hover {
    background: var(--color-primary);
    color: #000;
  }
</style>
