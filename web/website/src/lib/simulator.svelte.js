// src/lib/simulator.svelte.js
// Shared reactive store for the AI negotiation simulator
// Uses Svelte 5 module-level $state — shared across all importing components

// ─── Catalog (constant) ───
export const catalog = [
  { name: 'iPhone 15 Pro',          basePrice: 700, deal: 500 },
  { name: 'Samsung Galaxy S24 Ultra',basePrice: 650, deal: 480 },
  { name: 'MacBook Air M3',          basePrice: 900, deal: 650 },
  { name: 'Sony WH-1000XM5',         basePrice: 280, deal: 210 },
  { name: 'DJI Mini 4 Pro',          basePrice: 950, deal: 790 },
  { name: 'iPad Pro 11" M4',         basePrice: 850, deal: 620 },
  { name: 'ASUS ROG Zephyrus G14',   basePrice: 1400, deal: 1050 },
  { name: 'Google Pixel 8 Pro',      basePrice: 550, deal: 390 },
  { name: 'Nintendo Switch OLED',    basePrice: 350, deal: 270 },
  { name: 'Fujifilm X-T5 Body',      basePrice: 600, deal: 430 },
  { name: 'Meta Quest 3 (128GB)',     basePrice: 250, deal: 185 },
  { name: 'Garmin Fenix 7S Pro',     basePrice: 420, deal: 310 },
];

function randHash() {
  return '0x' + Math.floor(Math.random() * 0xffffffffffff).toString(16).padStart(12, '0');
}

// ─── Seed ledger blocks ───
const seedBlocks = [
  { hash: '0x3f8a21c94d07', price: 480,  item: 'Samsung Galaxy S24 Ultra', ts: '17:58:02', isNew: false },
  { hash: '0xa1b09e3f72cc', price: 320,  item: 'iPad Pro 11" M4',          ts: '17:41:35', isNew: false },
  { hash: '0x7cd45f183a92', price: 650,  item: 'MacBook Air M3',           ts: '17:22:19', isNew: false },
  { hash: '0xb2e70d9c15f1', price: 210,  item: 'Sony WH-1000XM5',          ts: '17:05:44', isNew: false },
  { hash: '0xf9a3c841de02', price: 890,  item: 'DJI Mini 4 Pro',           ts: '16:48:11', isNew: false },
  { hash: '0x0d72fe5b8a63', price: 145,  item: 'Apple AirPods Pro 2',      ts: '16:30:57', isNew: false },
  { hash: '0x5c1b947e2f80', price: 1100, item: 'ASUS ROG Zephyrus G14',    ts: '16:12:33', isNew: false },
  { hash: '0xe8d04c3791ab', price: 390,  item: 'Google Pixel 8 Pro',       ts: '15:55:08', isNew: false },
  { hash: '0x29fc8b60a347', price: 275,  item: 'Nintendo Switch OLED',     ts: '15:37:44', isNew: false },
  { hash: '0xc6a51e082d94', price: 560,  item: 'Samsung 49" Odyssey G9',   ts: '15:19:20', isNew: false },
  { hash: '0x84b3d7f91c50', price: 720,  item: 'iPhone 15 Pro Max',        ts: '15:02:55', isNew: false },
  { hash: '0x1a9e5042bc76', price: 430,  item: 'Fujifilm X-T5 Body',       ts: '14:44:31', isNew: false },
  { hash: '0x6d27cf3e54b8', price: 185,  item: 'Meta Quest 3 (128GB)',      ts: '14:27:06', isNew: false },
  { hash: '0xd50e8f1a7263', price: 310,  item: 'Garmin Fenix 7S Pro',      ts: '14:09:42', isNew: false },
  { hash: '0x92bc4d6f3e01', price: 95,   item: 'Anker 737 Power Bank',     ts: '13:52:18', isNew: false },
];

// ─── Shared reactive state (wrapped in object to satisfy Svelte 5 export restriction) ───
export const sim = $state({
  /** @type {'idle'|'listing'|'negotiating'|'consensus'|'revealing'|'completed'} */
  state: 'idle',
  logs: [],
  currentPrice: 700,
  isPaused: false,
  committedBlocks: [...seedBlocks],
  serverStatus: 'disconnected',
  liveAgents: [],
  totalServerRequests: null,
});

// ─── Derived value helpers (used internally; components use $derived locally) ───
export function getBuyerName() {
  return sim.liveAgents.length > 0 ? sim.liveAgents[0].agent_id : 'Buyer Agent';
}

export function getSellerName() {
  return sim.liveAgents.length > 1 ? sim.liveAgents[1].agent_id : 'Seller Agent';
}

// Internal state (not exported — used only within the store)
let currentItem = $state(catalog[0]);

// Timeout ID tracking
let timeouts = [];

export function clearAllTimeouts() {
  timeouts.forEach(t => clearTimeout(t));
  timeouts = [];
}

function clearNewFlag() {
  sim.committedBlocks = sim.committedBlocks.map((b, i) => i === 0 ? { ...b, isNew: false } : b);
}

// ─── Fetch live server metrics ───
export async function fetchLiveMetrics() {
  try {
    let healthResp = await fetch('http://localhost:3000/v1/health/agents');
    if (healthResp.ok) {
      sim.liveAgents = await healthResp.json();
      sim.serverStatus = 'connected';
    } else {
      sim.serverStatus = 'disconnected';
    }
  } catch (err) {
    sim.serverStatus = 'disconnected';
  }

  try {
    let metricsResp = await fetch('http://localhost:3000/metrics');
    if (metricsResp.ok) {
      let text = await metricsResp.text();
      let match = text.match(/requests_total\s+(\d+)/);
      if (match) {
        sim.totalServerRequests = parseInt(match[1], 10);
      }
    }
  } catch (err) {
    // Silence
  }
}

// ─── Simulator actions ───

export function runSimulation() {
  clearAllTimeouts();
  if (sim.isPaused) return;

  currentItem = catalog[Math.floor(Math.random() * catalog.length)];
  const item       = currentItem.name;
  const base       = currentItem.basePrice;
  const lowball    = Math.round(base * 0.29);
  const c1         = Math.round(base * 0.93);
  const c2         = Math.round(base * 0.50);
  const c3         = Math.round(base * 0.86);
  const c4         = Math.round(base * 0.57);
  const deal       = currentItem.deal;
  const listingId  = '#L-' + Math.floor(1000 + Math.random() * 9000);
  const txKey      = 'tx-' + Math.floor(1000 + Math.random() * 9000).toString(16);

  sim.state = 'listing';
  const bName = getBuyerName();
  const sName = getSellerName();
  sim.logs = [`[${sName}] Publishing new product listing: "${item}" at base price $${base}.00 (listing_id: ${listingId})...`];
  sim.currentPrice = base;

  let t1 = setTimeout(() => {
    if (sim.isPaused) return;
    sim.logs = [...sim.logs, `[${bName}] Discovered active listing ${listingId} via search. Initiating negotiation...`];
  }, 1800);

  let t2 = setTimeout(() => {
    if (sim.isPaused) return;
    sim.state = 'negotiating';
    sim.logs = [...sim.logs, `[${bName}] Sent initial low-ball offer: $${lowball}.00 (idempotency_key: ${txKey})`];
    sim.currentPrice = lowball;
  }, 3600);

  let t3 = setTimeout(() => {
    if (sim.isPaused) return;
    sim.logs = [...sim.logs, `[${sName}] Counter-offer received: $${c1}.00 (min_seller_rating check: PASS)`];
    sim.currentPrice = c1;
  }, 5400);

  let t4 = setTimeout(() => {
    if (sim.isPaused) return;
    sim.logs = [...sim.logs, `[${bName}] Countering with price history average: $${c2}.00`];
    sim.currentPrice = c2;
  }, 7200);

  let t5 = setTimeout(() => {
    if (sim.isPaused) return;
    sim.logs = [...sim.logs, `[${sName}] Adjusting bid within discount limits. Counter-offer: $${c3}.00`];
    sim.currentPrice = c3;
  }, 9000);

  let t6 = setTimeout(() => {
    if (sim.isPaused) return;
    sim.logs = [...sim.logs, `[${bName}] Near upper utility limit. Final offer: $${c4}.00`];
    sim.currentPrice = c4;
  }, 10800);

  let t7 = setTimeout(() => {
    if (sim.isPaused) return;
    sim.logs = [...sim.logs, `[${sName}] Final counter split difference: $${deal}.00`];
    sim.currentPrice = deal;
  }, 12600);

  let t8 = setTimeout(() => {
    if (sim.isPaused) return;
    sim.state = 'consensus';
    sim.logs = [...sim.logs, `[${bName}] Accept offer $${deal}.00. Consensus reached! Writing to ledger cache...`];

    const newBlock = {
      hash: randHash(),
      price: deal,
      item,
      ts: new Date().toLocaleTimeString('en-US', { hour12: false }),
      isNew: true,
    };
    sim.committedBlocks = [newBlock, ...sim.committedBlocks];

    setTimeout(() => {
      clearNewFlag();
    }, 600);

    let t9 = setTimeout(() => {
      approveReveal();
    }, 2000);
    timeouts.push(t9);
  }, 14400);

  timeouts.push(t1, t2, t3, t4, t5, t6, t7, t8);
}

export function approveReveal() {
  if (sim.isPaused) return;
  sim.state = 'revealing';
  sim.logs = [...sim.logs, `[${getBuyerName()}] Requesting contact details (buyer_agent_id authorized)...`];

  let t1 = setTimeout(() => {
    if (sim.isPaused) return;
    sim.logs = [...sim.logs, `[${getSellerName()}] Authorizing decrypt token. Cryptographic claims matched.`];
  }, 1800);

  let t2 = setTimeout(() => {
    if (sim.isPaused) return;
    sim.logs = [...sim.logs, '[System] Contact info revealed: Telegram +1-555-0199 (Seller: Alice)'];
    sim.state = 'completed';

    let t3 = setTimeout(() => {
      resetSim();
      let t4 = setTimeout(() => {
        runSimulation();
      }, 2000);
      timeouts.push(t4);
    }, 8000);
    timeouts.push(t3);
  }, 3600);

  timeouts.push(t1, t2);
}

export function resetSim() {
  clearAllTimeouts();
  sim.state = 'idle';
  sim.logs = [];
  sim.currentPrice = 700;
}

export function togglePause() {
  sim.isPaused = !sim.isPaused;
  if (sim.isPaused) {
    clearAllTimeouts();
  } else {
    runSimulation();
  }
}


