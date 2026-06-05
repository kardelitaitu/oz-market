# Website Refactoring Plan: Split `App.svelte` into Smaller Components

**Status:** Planning phase — not yet implemented  
**Target folder:** `web/website/src/lib/`

---

## 1. Current Situation

`App.svelte` is a single ~560-line component containing:
- Header with logo, navigation, theme swatches
- Simulator logic (state machine, timeouts, agent negotiation)
- SVG flow diagram (pure geometry with `animateMotion`)
- Live ledger block explorer
- Live server metrics fetcher
- Navigation tabs (home, device guide, docs)
- Footer

All state is declared at the top level with Svelte 5 `$state`/`$derived` runes.

---

## 2. Proposed Component Tree

```
App.svelte
├── Header.svelte
│   ├── Logo + nav buttons
│   └── ThemeSwitcher.svelte <-- extracted
├── MetricsBar.svelte          <-- extracted
├── Simulator.svelte           <-- extracted (biggest piece)
│   ├── AgentCard.svelte       <-- extracted (buyer/seller cards)
│   ├── FlowDiagram.svelte     <-- extracted (SVG)
│   └── LedgerExplorer.svelte  <-- extracted
├── TabContent (guide/docs remain inline or extracted later)
└── Footer.svelte
```

---

## 3. Shared State Store (`src/lib/simulator.svelte.js`)

Since the simulator logic is deeply interconnected, extract a shared store using Svelte 5's module-level `$state`:

```js
// src/lib/simulator.svelte.js
// Svelte 5 module-level $state — reactive across components

// --- Simulator state ---
export const simState       = $state('idle');
export const simLogs        = $state([]);
export const currentPrice   = $state(700);
export const currentItem    = $state(catalog[0]);
export const isPaused       = $state(false);

// --- Live metrics ---
export const serverStatus    = $state('disconnected');
export const liveAgents      = $state([]);
export const totalServerRequests = $state(null);

// --- Ledger ---
export const committedBlocks = $state([...historialBlocks]);
export const totalVolume     = $derived(committedBlocks.reduce(...));

// --- Derived agent names ---
export const buyerName  = $derived(liveAgents[0]?.agent_id ?? 'Buyer Agent');
export const sellerName = $derived(liveAgents[1]?.agent_id ?? 'Seller Agent');

// --- Catalog (constant) ---
export const catalog = [...];

// --- Actions ---
export function runSimulation() { ... }
export function approveReveal() { ... }
export function resetSim()      { ... }
export function togglePause()   { ... }
export async function fetchLiveMetrics() { ... }
export function clearAllTimeouts() { ... }

// --- Auto-start effect ---
$effect(() => {
  if (!isPaused) runSimulation();
  fetchLiveMetrics();
  const interval = setInterval(fetchLiveMetrics, 3000);
  return () => { clearAllTimeouts(); clearInterval(interval); };
});
```

This keeps all the orchestration in one place — no prop drilling for shared state.

---

## 4. Component Specifications

### 4.1 `ThemeSwitcher.svelte`

**Props:** none (reads/writes `currentTheme` via import)  
**Logic extracted:**
- `currentTheme` state (moved to store or kept local + exported)
- Theme data array
- `$effect()` for `document.body.setAttribute()` + localStorage

> **Note:** `currentTheme` is independent from the simulator. It can stay in `App.svelte` and be passed as a prop, or be lifted to its own small store `src/lib/theme.svelte.js`.

**Recommended:** Keep as a self-contained component with its own state. Use `onMount` for the localStorage read.

```svelte
<script>
  let currentTheme = $state(localStorage.getItem('oz-market-theme') || 'midnight');
  $effect(() => {
    document.body.setAttribute('data-theme', currentTheme);
    localStorage.setItem('oz-market-theme', currentTheme);
  });
</script>

<div class="theme-swatches" role="group" aria-label="Select Theme">
  <span class="theme-swatches-label">Theme</span>
  {#each themes as th}
    <button ... onclick={() => currentTheme = th.id}></button>
  {/each}
</div>
```

---

### 4.2 `MetricsBar.svelte`

**State source:** imports `serverStatus`, `liveAgents`, `totalServerRequests` from simulator store  
**Logic extracted:** The inline status bar markup (3 pill badges)

Simplifies to a ~40-line component.

---

### 4.3 `FlowDiagram.svelte`

**Props:** `simState` (string)  
**Logic extracted:**
- All SVG markup (nodes, lines, animated dots, gear SVG, consensus ring)
- The `node-glow` filter definition

Simplifies to a ~100-line pure display component. No script logic beyond the prop.

---

### 4.4 `LedgerExplorer.svelte`

**Props:** none (imports `committedBlocks`, `totalVolume` from store)  
**Logic extracted:**
- The `.ledger-explorer` div and its children
- Auto-scroll logic on block addition (via `$effect` watching `committedBlocks`)
- The badge + volume footer

**Note:** Needs to handle the `ledgerContainer` bind for scrolling. Can use `bind:this` internally.

---

### 4.5 `AgentCard.svelte`

**Props:** `agentName`, `agentTitle` (e.g. "buyer_negotiator"), `role` ("buyer"/"seller"), `isActive`, `glowColor`  
**Logic extracted:** The card divs for buyer and seller agents

Used twice in `Simulator.svelte`:
```svelte
<AgentCard name={buyerName} role="buyer" isActive={buyerActive} />
<AgentCard name={sellerName} role="seller" isActive={sellerActive} />
```

---

### 4.6 `Simulator.svelte`

**State source:** imports everything from simulator store  
**Logic extracted:**
- The `simLogs` rendering with `logParts`/`logColor` helpers
- The agent cards row (uses `AgentCard`)
- The `FlowDiagram` component
- The `LedgerExplorer` component
- The action buttons row (run/request reveal/pause)
- The split layout grid

This becomes the orchestrator component — ~120 lines instead of ~400.

---

### 4.7 `App.svelte` (post-refactor)

Becomes a thin shell:
```svelte
<script>
  import Header from './lib/Header.svelte';
  import MetricsBar from './lib/MetricsBar.svelte';
  import Simulator from './lib/Simulator.svelte';
  import Footer from './lib/Footer.svelte';

  let currentTab = $state('home');
  let deviceTab = $state('server');
</script>

<Header {currentTab} on:tabchange={...} />
<main class="container">
  {#if currentTab === 'home'}
    <MetricsBar />
    <Simulator />
    <!-- value pillars, benchmark table -->
  {:else if currentTab === 'guide'}
    <!-- guide content -->
  {:else}
    <!-- docs content -->
  {/if}
</main>
<Footer />
```

This brings `App.svelte` from ~560 lines to ~100-120 lines.

---

## 5. File Structure After Refactor

```
web/website/src/
├── main.js
├── App.svelte                    # ~100 lines (thin shell)
├── global.css                    # unchanged
├── lib/
│   ├── simulator.svelte.js       # shared state store (~250 lines)
│   ├── Header.svelte             # logo + nav + ThemeSwitcher  (~50 lines)
│   ├── ThemeSwitcher.svelte      # theme swatches               (~50 lines)
│   ├── MetricsBar.svelte         # server status pills           (~40 lines)
│   ├── Simulator.svelte          # orchestrator                 (~120 lines)
│   ├── AgentCard.svelte          # buyer/seller card            (~40 lines)
│   ├── FlowDiagram.svelte        # SVG architecture flow        (~100 lines)
│   └── LedgerExplorer.svelte     # ledger block explorer        (~80 lines)
└── assets/
```

---

## 6. Implementation Order (dependency-safe)

| Step | Component | Why this order |
|------|-----------|---------------|
| 1 | `src/lib/simulator.svelte.js` | All other components depend on it |
| 2 | `ThemeSwitcher.svelte` | Independent, no store needed |
| 3 | `AgentCard.svelte` | Pure display, no store needed |
| 4 | `FlowDiagram.svelte` | Only needs `simState` prop |
| 5 | `LedgerExplorer.svelte` | Needs store |
| 6 | `MetricsBar.svelte` | Needs store |
| 7 | `Simulator.svelte` | Combines AgentCard + FlowDiagram + LedgerExplorer |
| 8 | `App.svelte` (rewrite) | Integrates all components, removes duplicated code |
| 9 | Run E2E tests & dev server | Validate everything still works |

---

## 7. Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Svelte 5 `$state` in `.svelte.js` files works differently than in `.svelte` files | Verified: Svelte 5 supports `$state` in `.svelte.js` modules since v5.0 |
| `animateMotion` behavior fragile across component boundaries | Test after refactor — SVG should remain identical |
| `$effect` auto-start behavior changes with store location | The store's `$effect` fires on import — test that autoplay still starts on page load |
| `ledgerContainer` scroll binding breaks after extraction | `LedgerExplorer` manages its own `bind:this` |
| Timeouts management across components | All timeout logic stays in the single store file |

---

## 8. Testing After Refactor

```bash
cd web/website
npm run build              # Ensure build succeeds
npm run test:e2e           # Run existing Playwright tests
npm run dev                # Manual smoke test
```

Expected: all 4 existing Playwright tests pass unchanged. The components are a pure refactor — no behavior changes.

---

## 9. Future Opportunities

After splitting, the components are independently testable. Each could get a `.test.js` file:

- `LedgerExplorer`: test block rendering, badge counts
- `ThemeSwitcher`: test theme toggle, localStorage persistence
- `Simulator`: test state transitions, log output
- `FlowDiagram`: snapshot test SVG output per state
