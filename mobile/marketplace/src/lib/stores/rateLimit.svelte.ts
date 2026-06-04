import { getRateLimits } from '$lib/api/commands';
import type { RateLimitInfo } from '$lib/api/commands';

let _limits = $state<RateLimitInfo[]>([]);
let _pollTimer: ReturnType<typeof setInterval> | undefined;

/**
 * Reactive rate limit store. Call `startPolling()` when the app needs
 * live rate limit data (e.g., on pages with action buttons).
 */
export const rateLimits = {
  get all(): RateLimitInfo[] {
    return _limits;
  },

  /** Get the entry for a specific action, or undefined. */
  forAction(action: string): RateLimitInfo | undefined {
    return _limits.find((l) => l.action === action);
  },

  /** True if any action is exhausted (remaining === 0). */
  get anyExhausted(): boolean {
    return _limits.some((l) => l.remaining === 0);
  },

  /** True if any action is running low (remaining / limit < 0.2). */
  get anyLow(): boolean {
    return _limits.some((l) => l.limit > 0 && l.remaining / l.limit < 0.2);
  },

  startPolling(intervalMs = 15000): void {
    if (_pollTimer) return;
    const poll = async () => {
      try {
        _limits = await getRateLimits();
      } catch {
        // silently ignore — backend may be unreachable
      }
    };
    poll();
    _pollTimer = setInterval(poll, intervalMs);
  },

  stopPolling(): void {
    if (_pollTimer) {
      clearInterval(_pollTimer);
      _pollTimer = undefined;
    }
  },
};
