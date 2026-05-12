/**
 * SessionCachePolicy
 *
 * Single instance policy that enforces a bounded number of warm sessions and
 * demotes idle sessions from WARM -> COLD after a configurable timeout.
 *
 * Responsibilities:
 * - Track per-tab lastActiveAt and explicit tier (hot/warm/cold)
 * - Expose simple lifecycle hooks: onOpenTab, onCloseTab, onActivate, touch
 * - Periodically enforce LRU + idle demotion
 * - Emit optional callbacks for external actors to react (lightweight)
 *
 * This implementation is intentionally conservative (no UI updates produced).
 */

import EditorSessionStore from '@/stores/EditorSessionStore';
import editorViewHost from '@/lib/session/EditorViewHost';
import { stateCache } from '@/components/editor/editorEngine';
import documentStore from '@/stores/DocumentStore';
import { debug, isDebug } from '@/lib/logger';

type Tier = 'hot' | 'warm' | 'cold';

type Listener = (tabId: string, tier: Tier) => void;

class SessionCachePolicy {
  private tiers: Map<string, Tier>;
  private lastActive: Map<string, number>;
  private listeners: Set<Listener>;
  private checkInterval: number;
  private maxWarm: number;
  private warmIdleMs: number;
  private timerId: number | null;

  constructor() {
    this.tiers = new Map();
    this.lastActive = new Map();
    this.listeners = new Set();
    // run enforcement every 10s by default
    this.checkInterval = 10_000;
    // keep at most this many warm sessions
    this.maxWarm = 8;
    // demote warm -> cold after this many ms of inactivity
    this.warmIdleMs = 60 * 1000; // 60s
    this.timerId = null;
    this.startTimer();
  }

  private startTimer() {
    if (this.timerId != null) return;
    this.timerId = window.setInterval(() => {
      try {
        this.enforcePolicy();
      } catch (e) {
        // swallow to avoid breaking host app (debug-only)
        try { debug('[session-cache] enforcement error', String(e)); } catch {}
      }
    }, this.checkInterval);
  }

  private stopTimer() {
    if (this.timerId != null) {
      window.clearInterval(this.timerId);
      this.timerId = null;
    }
  }

  onOpenTab(tabId: string) {
    // new tabs start as warm by default; activation will promote to hot
    this.tiers.set(tabId, 'warm');
    this.lastActive.set(tabId, Date.now());
    this.emit(tabId, 'warm');
    this.enforcePolicy();
  }

  onCloseTab(tabId: string) {
    this.tiers.delete(tabId);
    this.lastActive.delete(tabId);
    this.emit(tabId, 'cold');
    this.enforcePolicy();
  }

  onActivate(tabId: string) {
    // demote existing hot tabs to warm (only a single hot)
    for (const [id, tier] of this.tiers.entries()) {
      if (id !== tabId && tier === 'hot') {
        this.tiers.set(id, 'warm');
        this.emit(id, 'warm');
      }
    }
    this.tiers.set(tabId, 'hot');
    this.lastActive.set(tabId, Date.now());
    this.emit(tabId, 'hot');
    this.enforcePolicy();
  }

  touch(tabId: string) {
    this.lastActive.set(tabId, Date.now());
  }

  getTier(tabId: string): Tier {
    return this.tiers.get(tabId) ?? 'cold';
  }

  subscribe(fn: Listener) {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  private emit(tabId: string, tier: Tier) {
    for (const l of Array.from(this.listeners)) {
      try { l(tabId, tier); } catch {}
    }
  }

  private enforcePolicy() {
    // Demote warm -> cold if idle
    const now = Date.now();
    const warmCandidates: Array<{ id: string; last: number }> = [];
    for (const [id, tier] of this.tiers.entries()) {
      if (tier === 'warm') {
        const last = this.lastActive.get(id) ?? 0;
        if (now - last > this.warmIdleMs) {
          // demote immediately
          this.tiers.set(id, 'cold');
          this.emit(id, 'cold');
          // Enforce demotion side-effects: compact session, destroy live view, clear engine state cache.
          try { EditorSessionStore.compactToCold(id); } catch {}
          try { editorViewHost.destroyIfFor(id); } catch {}
          try {
            // stateCache keys are documentIds. Resolve tabId -> documentId via EditorSessionStore snapshot.
            const snap = EditorSessionStore.getSnapshot(id);
            const docId = snap?.documentId ?? null;
            if (docId && stateCache && typeof (stateCache as any).delete === 'function') {
              (stateCache as any).delete(docId);
            }
          } catch {}
        } else {
          warmCandidates.push({ id, last });
        }
      }
    }

    // Enforce maxWarm by LRU (least recently active warm -> cold)
    if (warmCandidates.length > this.maxWarm) {
      warmCandidates.sort((a, b) => a.last - b.last); // oldest first
      let toDemote = warmCandidates.length - this.maxWarm;
      for (const c of warmCandidates) {
        if (toDemote <= 0) break;
        this.tiers.set(c.id, 'cold');
        this.emit(c.id, 'cold');
        // Enforce demotion side-effects for each demoted session.
        try { EditorSessionStore.compactToCold(c.id); } catch {}
        try { editorViewHost.destroyIfFor(c.id); } catch {}
        try {
          // Resolve tabId -> documentId before deleting engine cache entries.
          const snap = EditorSessionStore.getSnapshot(c.id);
          const docId = snap?.documentId ?? null;
          if (docId && stateCache && typeof (stateCache as any).delete === 'function') {
            (stateCache as any).delete(docId);
          }
        } catch {}
        toDemote--;
      }
    }

    // Update runtime diagnostics (debug-only, lightweight)
    try {
      const warmCount = Array.from(this.tiers.values()).filter((t) => t === 'warm').length;
      const coldCount = Array.from(this.tiers.values()).filter((t) => t === 'cold').length;
      const live = typeof (editorViewHost as any).getLiveCount === 'function' ? (editorViewHost as any).getLiveCount() : 0;
      if (isDebug()) {
        const rs: any = (window as any).__zaroxi_runtime_stats || {};
        rs.warmCount = warmCount;
        rs.coldCount = coldCount;
        rs.liveViews = live;
        rs.lastEnforce = Date.now();
        (window as any).__zaroxi_runtime_stats = rs;
      }
    } catch {}
  }

  // Administrative helpers
  setMaxWarm(n: number) {
    this.maxWarm = Math.max(1, n);
    this.enforcePolicy();
  }

  setWarmIdleMs(ms: number) {
    this.warmIdleMs = Math.max(1000, ms);
    this.enforcePolicy();
  }
}

const sessionCache = new SessionCachePolicy();
export default sessionCache;
