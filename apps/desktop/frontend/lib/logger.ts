/**
 * Lightweight logger utility for controlled diagnostics.
 *
 * - Debugging is OFF by default.
 * - Enable transient diagnostics by setting `localStorage.setItem('zaroxi:debug','1')`
 *   or by setting `window.__ZAROXI_DEBUG = true` in the console during development.
 *
 * API:
 *   isDebug(): boolean
 *   debug(...args)
 *   info(...args)      // debug-only info
 *   warn(...args)      // always logs (kept concise)
 *   error(...args)     // always logs (kept concise)
 *   incrementStat(name, delta)   // update lightweight numeric stat under window.__zaroxi_cm_stats (debug-only)
 *   setStat(name, value)         // set a named stat under window.__zaroxi_runtime_stats (debug-only)
 *   setInspect(name, fn)         // expose an inspect helper on window (debug-only)
 *   setMountError(err)           // set a small mount-error object (debug-only)
 *
 * This centralizes debug gating so files no longer scatter `console.*` calls.
 */

const DEBUG_KEY = 'zaroxi:debug';

export function isDebug(): boolean {
  try {
    if (typeof window === 'undefined') return false;
    // Explicit runtime toggle (useful from the dev console)
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    if ((window as any).__ZAROXI_DEBUG === true) return true;
    // LocalStorage toggle (persisted)
    try {
      return window.localStorage?.getItem(DEBUG_KEY) === '1';
    } catch {
      return false;
    }
  } catch {
    return false;
  }
}

export function debug(...args: any[]) {
  if (!isDebug()) return;
  try {
    // Prefix for easier grep
    // eslint-disable-next-line no-console
    console.debug('[zaroxi]', ...args);
  } catch {}
}

export function info(...args: any[]) {
  if (!isDebug()) return;
  try {
    // eslint-disable-next-line no-console
    console.info('[zaroxi]', ...args);
  } catch {}
}

export function warn(...args: any[]) {
  try {
    // eslint-disable-next-line no-console
    console.warn('[zaroxi]', ...args);
  } catch {}
}

export function error(...args: any[]) {
  try {
    // eslint-disable-next-line no-console
    console.error('[zaroxi]', ...args);
  } catch {}
}

/**
 * Increment a small numeric stat used for lightweight runtime counters.
 * Stored under window.__zaroxi_cm_stats when debug is enabled.
 */
export function incrementStat(name: string, delta: number) {
  if (!isDebug()) return;
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const w: any = window;
    w.__zaroxi_cm_stats = w.__zaroxi_cm_stats || {};
    w.__zaroxi_cm_stats[name] = (w.__zaroxi_cm_stats[name] || 0) + delta;
  } catch {}
}

/**
 * Set a runtime stat (used for cached sizes, counts, etc.).
 * Stored under window.__zaroxi_runtime_stats when debug is enabled.
 */
export function setStat(name: string, value: any) {
  if (!isDebug()) return;
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const w: any = window;
    w.__zaroxi_runtime_stats = w.__zaroxi_runtime_stats || {};
    w.__zaroxi_runtime_stats[name] = value;
  } catch {}
}

/**
 * Expose a small inspection helper on window under the provided name.
 * Only set in debug mode.
 */
export function setInspect(name: string, fn: Function) {
  if (!isDebug()) return;
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (window as any)[name] = fn;
  } catch {}
}

/**
 * Record a small mount error object for diagnostic use only (debug-mode).
 */
export function setMountError(err: any) {
  if (!isDebug()) return;
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (window as any).__codemirror_mount_error = {
      message: String((err as any)?.message ?? err),
      stack: (err as any)?.stack ?? null,
    };
  } catch {}
}
