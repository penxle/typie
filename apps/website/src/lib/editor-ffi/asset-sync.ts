import { ASSET_MESSAGE_MAX_ITEMS } from '$lib/sync/protocol';
import type { AssetStateEntry } from '$lib/sync/protocol';

export type AssetSyncOptions = {
  pull: (requestId: string, ids: string[]) => void;
  apply: (entries: AssetStateEntry[]) => void;
  basePollMs?: number;
  maxPollMs?: number;
  debounceMs?: number;
  maxIdsPerPull?: number;
};

export type AssetSync = {
  update: (referencedIds: string[]) => void;
  invalidate: (ids: string[]) => void;
  receive: (requestId: string, entries: AssetStateEntry[], final: boolean) => void;
  repullReferenced: (referencedIds: string[]) => void;
  dispose: () => void;
};

type AssetState = AssetStateEntry['state'];

let instanceSeq = 0;

// image/file 해석 상태 기계. 서버에는 버전이 없으므로 순서 판정은 전적으로 여기서 한다:
// pull마다 requestId를 발급해 id별 최신 requestId를 기록하고, 그보다 낡은 응답의 entry는 버린다.
// `missing`은 확정된 부재가 아니라 "지금은 해석 불가"이므로 문서를 건드리지 않고 계속 재-pull한다.
export const createAssetSync = ({
  pull,
  apply,
  basePollMs = 60_000,
  maxPollMs = 300_000,
  debounceMs = 100,
  maxIdsPerPull = ASSET_MESSAGE_MAX_ITEMS,
}: AssetSyncOptions): AssetSync => {
  const instanceId = `s${++instanceSeq}`;
  let requestSeq = 0;

  let referenced = new Set<string>();
  const states = new Map<string, AssetState>();
  const latestRequestIds = new Map<string, string>();
  const awaiting = new Set<string>();
  const queued = new Set<string>();

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let pollTimer: ReturnType<typeof setTimeout> | null = null;
  let pollDelay = basePollMs;
  let disposed = false;

  const nonReadyIds = () => [...referenced].filter((id) => states.get(id) !== 'ready');

  const dispatch = (ids: string[]) => {
    for (let index = 0; index < ids.length; index += maxIdsPerPull) {
      const chunk = ids.slice(index, index + maxIdsPerPull);
      const requestId = `${instanceId}-${++requestSeq}`;
      for (const id of chunk) {
        latestRequestIds.set(id, requestId);
        awaiting.add(id);
      }
      pull(requestId, chunk);
    }
  };

  const clearDebounce = () => {
    if (debounceTimer === null) return;
    clearTimeout(debounceTimer);
    debounceTimer = null;
  };

  const flush = () => {
    debounceTimer = null;
    if (disposed) return;
    const ids = [...queued].filter((id) => referenced.has(id));
    queued.clear();
    if (ids.length > 0) dispatch(ids);
  };

  const enqueue = (ids: Iterable<string>) => {
    for (const id of ids) queued.add(id);
    if (queued.size === 0 || debounceTimer !== null) return;
    debounceTimer = setTimeout(flush, debounceMs);
  };

  const clearPoll = () => {
    if (pollTimer === null) return;
    clearTimeout(pollTimer);
    pollTimer = null;
  };

  const ensurePoll = () => {
    if (disposed || pollTimer !== null || nonReadyIds().length === 0) return;
    pollTimer = setTimeout(() => {
      pollTimer = null;
      const ids = nonReadyIds();
      if (ids.length === 0) {
        pollDelay = basePollMs;
        return;
      }

      dispatch(ids);
      pollDelay = Math.min(pollDelay * 2, maxPollMs);
      ensurePoll();
    }, pollDelay);
  };

  const resetPoll = () => {
    pollDelay = basePollMs;
    clearPoll();
    ensurePoll();
  };

  const retire = (id: string) => {
    states.delete(id);
    latestRequestIds.delete(id);
    awaiting.delete(id);
  };

  const setReferenced = (referencedIds: string[]) => {
    const next = new Set(referencedIds);
    for (const id of referenced) {
      if (!next.has(id)) retire(id);
    }
    referenced = next;
  };

  return {
    update(referencedIds) {
      if (disposed) return;
      setReferenced(referencedIds);
      enqueue([...referenced].filter((id) => !states.has(id) && !awaiting.has(id)));
      ensurePoll();
    },

    invalidate(ids) {
      if (disposed) return;
      enqueue(ids);
      ensurePoll();
    },

    receive(requestId, entries, final) {
      if (disposed) return;

      const accepted: AssetStateEntry[] = [];
      for (const entry of entries) {
        if (latestRequestIds.get(entry.id) !== requestId || !referenced.has(entry.id)) continue;
        awaiting.delete(entry.id);
        const previous = states.get(entry.id);
        // 상태가 같으면 apply까지 가지 않는다(폴링마다 같은 pending/missing으로 리렌더하지 않기 위함) —
        // 같은 상태에서 payload만 바뀐 경우는 반영되지 않는다.
        if (previous === entry.state || (previous === 'ready' && entry.state !== 'ready')) continue;
        states.set(entry.id, entry.state);
        accepted.push(entry);
      }

      // 응답 자체가 오지 않을 수 있으므로 미결 표시는 이 프레임이 끝날 때만 정리한다
      // (그래도 고착되지 않는다 — 폴링은 미결 여부와 무관하게 non-ready 전부를 재-pull한다).
      if (final) {
        for (const [id, issued] of latestRequestIds) {
          if (issued === requestId) awaiting.delete(id);
        }
      }

      if (accepted.length > 0) {
        apply(accepted);
        resetPoll();
      } else {
        ensurePoll();
      }
    },

    repullReferenced(referencedIds) {
      if (disposed) return;
      setReferenced(referencedIds);
      clearDebounce();
      queued.clear();
      const ids = nonReadyIds();
      if (ids.length > 0) dispatch(ids);
      resetPoll();
    },

    dispose() {
      disposed = true;
      clearDebounce();
      clearPoll();
      referenced.clear();
      states.clear();
      latestRequestIds.clear();
      awaiting.clear();
      queued.clear();
    },
  };
};
