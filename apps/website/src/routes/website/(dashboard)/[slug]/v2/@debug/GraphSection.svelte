<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import dayjs from 'dayjs';
  import { graphql } from '$mearie';
  import type { ClientCommitInput, DebugSnapshot } from './types';

  type CommitLite = {
    id: string;
    hash: string;
    parent: { id: string; hash: string } | null;
    secondParent: { id: string; hash: string } | null;
    committedAt: string;
    pushedAt: string | null;
    device: { id: string; name: string; isCurrent: boolean } | null;
    user: { id: string } | null;
  };

  type RowMeta = {
    commit: CommitLite;
    lane: 0 | 1;
    isMerge: boolean;
    lane1Line: 'full' | 'topHalf' | null;
    isLCA: boolean;
  };

  type Props = {
    slug: string;
    snapshot: DebugSnapshot;
    onSelectCommit?: (id: string) => void;
    loadedCommits?: { id: string; hash: string }[];
  };

  /* eslint-disable no-useless-assignment -- $bindable() defaults are used by Svelte */
  let { slug, snapshot, onSelectCommit, loadedCommits = $bindable([]) }: Props = $props();
  /* eslint-enable no-useless-assignment */

  type Vars = { slug: string; after: string | null; limit: number };
  const PAGE = 50;

  let commits = $state<CommitLite[]>([]);
  let vars = $state<Vars>({ slug, after: null, limit: PAGE });
  let exhausted = $state(false);

  const query = createQuery(
    graphql(`
      query DocumentEditorV2_Debug_Commits($slug: String!, $after: ID, $limit: Int!) {
        document(slug: $slug) {
          id
          commits(after: $after, limit: $limit) {
            id
            hash
            parent {
              id
              hash
            }
            secondParent {
              id
              hash
            }
            committedAt
            pushedAt
            device {
              id
              name
              isCurrent
            }
            user {
              id
            }
          }
        }
      }
    `),
    () => vars,
  );

  const lastError = $derived(query.error ? String(query.error) : null);

  $effect(() => {
    const data = query.data;
    if (!data) return;

    const fetched = data.document.commits as CommitLite[];

    if (vars.after !== null && fetched.length < vars.limit) {
      exhausted = true;
    }

    const existing: Record<string, true> = {};
    for (const c of commits) existing[c.id] = true;
    const novel = fetched.filter((c) => !existing[c.id]);
    if (novel.length === 0) return;

    if (vars.after === null) {
      commits = [...novel, ...commits];
    } else {
      commits = [...commits, ...novel];
    }
  });

  $effect(() => {
    loadedCommits = commits.map((c) => ({ id: c.id, hash: c.hash }));
  });

  let knownHead: string | null = null;
  $effect(() => {
    const head = snapshot.serverHeadHash;
    if (!head || head === knownHead) {
      if (head) knownHead = head;
      return;
    }
    const isInitial = knownHead === null;
    knownHead = head;
    if (isInitial) return; // initial fetch is driven by mount-time vars

    if (vars.after === null) {
      query.refetch();
    } else {
      vars = { slug, after: null, limit: PAGE };
    }
  });

  function assignLanes(commits: readonly CommitLite[]): RowMeta[] {
    const out: RowMeta[] = commits.map((c) => ({
      commit: c,
      lane: 0,
      isMerge: false,
      lane1Line: null,
      isLCA: false,
    }));

    const hashToIdx: Record<string, number | undefined> = {};
    for (const [i, m] of out.entries()) {
      hashToIdx[m.commit.hash] = i;
    }

    for (const m of out) {
      if (!m.commit.secondParent || !m.commit.parent) continue;

      const firstParentAncestors: Record<string, true> = {};
      let fp: string | undefined = m.commit.parent.hash;
      while (fp) {
        firstParentAncestors[fp] = true;
        const idx: number | undefined = hashToIdx[fp];
        if (idx === undefined) break;
        fp = out[idx].commit.parent?.hash;
      }

      const lane1Hashes: Record<string, true> = {};
      let sp: string | undefined = m.commit.secondParent.hash;
      let lcaHash: string | null = null;
      while (sp) {
        if (firstParentAncestors[sp]) {
          lcaHash = sp;
          break;
        }
        lane1Hashes[sp] = true;
        const idx: number | undefined = hashToIdx[sp];
        if (idx === undefined) break;
        sp = out[idx].commit.parent?.hash;
      }

      if (!lcaHash) continue;

      const spStartIdx = hashToIdx[m.commit.secondParent.hash];
      const lcaIdx = hashToIdx[lcaHash];
      if (spStartIdx === undefined || lcaIdx === undefined || lcaIdx < spStartIdx) continue;

      m.isMerge = true;

      for (let k = spStartIdx; k <= lcaIdx; k++) {
        if (k === lcaIdx) {
          out[k].isLCA = true;
        } else {
          out[k].lane1Line = 'full';
          if (lane1Hashes[out[k].commit.hash]) {
            out[k].lane = 1;
          }
        }
      }
    }

    return out;
  }

  const rowMetas = $derived(assignLanes(commits));

  function loadMore() {
    if (query.loading || exhausted) return;
    const last = commits.at(-1);
    if (!last) return;
    vars = { slug, after: last.id, limit: PAGE };
  }

  let prevLocalChain: readonly ClientCommitInput[] = [];
  let inflights = $state<readonly ClientCommitInput[]>([]);

  $effect(() => {
    const cur = snapshot.outbox.map((e) => e.commit);
    const present: Record<string, true> = {};
    for (const c of cur) present[c.commitHash] = true;
    const departed = prevLocalChain.filter((c) => !present[c.commitHash]);
    prevLocalChain = [...cur];
    if (departed.length > 0) {
      inflights = [...inflights, ...departed];
    }
  });

  $effect(() => {
    if (inflights.length === 0) return;
    const absorbed: Record<string, true> = {};
    for (const c of commits) absorbed[c.hash] = true;
    const remaining = inflights.filter((c) => !absorbed[c.commitHash]);
    if (remaining.length !== inflights.length) {
      inflights = remaining;
    }
  });

  const localChainReversed = $derived([...snapshot.outbox.map((e) => e.commit).toReversed(), ...inflights.toReversed()]);

  function fmtTime(iso: string): string {
    return dayjs(iso).format('HH:mm:ss');
  }

  // IntersectionObserver auto-paginate. The default root (viewport) doesn't
  // notice inner-scroll containers, so we anchor root to the scroll element.
  let scrollContainerEl = $state<HTMLDivElement>();
  let sentinelEl = $state<HTMLDivElement>();

  $effect(() => {
    if (!scrollContainerEl || !sentinelEl) return;
    const root = scrollContainerEl;
    const target = sentinelEl;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) loadMore();
      },
      { root, rootMargin: '100px' },
    );
    observer.observe(target);
    return () => observer.disconnect();
  });
</script>

<section
  class={css({
    flexGrow: '1',
    flexShrink: '1',
    minHeight: '0',
    paddingX: '12px',
    paddingY: '8px',
    borderBottomWidth: '1px',
    borderBottomColor: 'border.subtle',
    display: 'flex',
    flexDirection: 'column',
    overflow: 'hidden',
  })}
>
  <header
    class={css({
      display: 'flex',
      justifyContent: 'space-between',
      fontWeight: 'semibold',
      fontSize: '10px',
      letterSpacing: '0.04em',
      color: 'text.muted',
      marginBottom: '6px',
    })}
  >
    COMMITS
  </header>

  <div bind:this={scrollContainerEl} class={css({ flexGrow: '1', flexShrink: '1', minHeight: '0', overflowY: 'auto' })}>
    <ul class={css({ listStyle: 'none', paddingLeft: '0' })}>
      {#each localChainReversed as c, localIdx (c.commitHash)}
        <li
          class={css({
            position: 'relative',
            height: '28px',
            display: 'grid',
            gridTemplateColumns: '56px 1fr',
            alignItems: 'center',
          })}
        >
          <div class={css({ position: 'relative', height: 'full' })}>
            <div
              class={css({
                position: 'absolute',
                top: localIdx === 0 ? '14px' : '0',
                bottom: '0',
                left: '13px',
                width: '2px',
                backgroundImage: '[linear-gradient(to bottom, token(colors.palette.orange) 50%, transparent 50%)]',
                backgroundSize: '[2px 6px]',
                backgroundRepeat: 'repeat-y',
              })}
            ></div>
            <div
              class={css({
                position: 'absolute',
                left: '10px',
                top: '1/2',
                translate: 'auto',
                translateY: '-1/2',
                width: '8px',
                height: '8px',
                borderRadius: 'full',
                backgroundColor: 'white',
                borderWidth: '2px',
                borderColor: 'palette.orange',
              })}
            ></div>
          </div>
          <div
            class={css({
              fontFamily: 'mono',
              fontSize: '10px',
              display: 'flex',
              minWidth: '0',
              gap: '[0.5em]',
              color: 'text.muted',
              overflow: 'hidden',
              position: 'relative',
              pointerEvents: 'none',
            })}
          >
            <span class={css({ color: 'text.default' })}>
              {c.commitHash.slice(0, 8)}
            </span>
            ·
            <span>
              {fmtTime(c.committedAt)}
            </span>
            ·
            <span class={css({ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>(local)</span>
          </div>
        </li>
      {/each}

      {#each rowMetas as meta, idx (meta.commit.id)}
        {@const isFirst = idx === 0}
        {@const isLast = idx === rowMetas.length - 1}
        {@const hasLocalStack = localChainReversed.length > 0}
        <li
          class={css({
            position: 'relative',
            height: '28px',
            display: 'grid',
            gridTemplateColumns: '56px 1fr',
            alignItems: 'center',
            cursor: 'pointer',
            _hover: { backgroundColor: 'surface.subtle' },
          })}
        >
          <button
            class={css({
              position: 'absolute',
              inset: '0',
              cursor: 'pointer',
              backgroundColor: 'transparent',
              border: 'none',
              padding: '0',
            })}
            aria-label={`Commit ${meta.commit.hash.slice(0, 8)}`}
            onclick={() => onSelectCommit?.(meta.commit.id)}
            type="button"
          ></button>

          <div class={css({ position: 'relative', height: 'full', pointerEvents: 'none' })}>
            {#if !isFirst}
              <div
                class={css({
                  position: 'absolute',
                  top: '0',
                  height: '14px',
                  left: '13px',
                  width: '2px',
                  backgroundColor: 'palette.orange',
                })}
              ></div>
            {:else if hasLocalStack}
              <div
                class={css({
                  position: 'absolute',
                  top: '0',
                  height: '14px',
                  left: '13px',
                  width: '2px',
                  backgroundImage: '[linear-gradient(to bottom, token(colors.palette.orange) 50%, transparent 50%)]',
                  backgroundSize: '[2px 6px]',
                  backgroundRepeat: 'repeat-y',
                })}
              ></div>
            {/if}

            {#if !isLast}
              <div
                class={css({
                  position: 'absolute',
                  top: '14px',
                  bottom: '0',
                  left: '13px',
                  width: '2px',
                  backgroundColor: 'palette.orange',
                })}
              ></div>
            {/if}

            {#if meta.lane1Line === 'full'}
              <div
                class={css({ position: 'absolute', top: '0', bottom: '0', left: '37px', width: '2px', backgroundColor: 'palette.yellow' })}
              ></div>
            {/if}

            {#if meta.isMerge}
              <div
                class={css({
                  position: 'absolute',
                  left: '10px',
                  top: '1/2',
                  translate: 'auto',
                  translateY: '-1/2',
                  width: '8px',
                  height: '8px',
                  borderRadius: 'full',
                  backgroundColor: 'palette.yellow',
                })}
              ></div>
            {:else if meta.lane === 1}
              <div
                class={css({
                  position: 'absolute',
                  left: '34px',
                  top: '1/2',
                  translate: 'auto',
                  translateY: '-1/2',
                  width: '8px',
                  height: '8px',
                  borderRadius: 'full',
                  backgroundColor: 'palette.yellow',
                })}
              ></div>
            {:else}
              <div
                class={css({
                  position: 'absolute',
                  left: '10px',
                  top: '1/2',
                  translate: 'auto',
                  translateY: '-1/2',
                  width: '8px',
                  height: '8px',
                  borderRadius: 'full',
                  backgroundColor: 'palette.orange',
                })}
              ></div>
            {/if}

            {#if meta.isMerge}
              <div
                class={css({
                  position: 'absolute',
                  left: '17px',
                  top: '13px',
                  width: '22px',
                  height: '15px',
                  borderTopWidth: '2px',
                  borderRightWidth: '2px',
                  borderTopColor: 'palette.yellow',
                  borderRightColor: 'palette.yellow',
                  borderTopRightRadius: '14px',
                })}
              ></div>
            {/if}

            {#if meta.isLCA}
              <div
                class={css({
                  position: 'absolute',
                  left: '18px',
                  top: '0',
                  width: '21px',
                  height: '15px',
                  borderBottomWidth: '2px',
                  borderRightWidth: '2px',
                  borderBottomColor: 'palette.yellow',
                  borderRightColor: 'palette.yellow',
                  borderBottomRightRadius: '14px',
                })}
              ></div>
            {/if}
          </div>

          <div
            class={css({
              fontFamily: 'mono',
              fontSize: '10px',
              display: 'flex',
              minWidth: '0',
              gap: '[0.5em]',
              color: 'text.muted',
              overflow: 'hidden',
              position: 'relative',
              pointerEvents: 'none',
            })}
          >
            <span class={css({ color: 'text.default' })}>
              {meta.commit.hash.slice(0, 8)}
            </span>
            ·
            <span>
              {fmtTime(meta.commit.committedAt)}
            </span>
            ·
            <span class={css({ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>
              {meta.commit.device?.name ?? '(empty)'}{meta.commit.device?.isCurrent ? ' (this)' : ''}
            </span>
          </div>
        </li>
      {/each}
    </ul>

    {#if lastError}
      <div class={css({ color: 'palette.red', fontSize: '10px', paddingY: '4px' })}>
        {lastError}
        <button
          class={css({
            marginLeft: '8px',
            cursor: 'pointer',
            color: 'palette.blue',
            backgroundColor: 'transparent',
            border: 'none',
            padding: '0',
          })}
          onclick={loadMore}
          type="button"
        >
          retry
        </button>
      </div>
    {:else if !exhausted}
      <div bind:this={sentinelEl} class={css({ paddingY: '6px', textAlign: 'center', fontSize: '10px', color: 'text.faint' })}>
        {query.loading ? '…' : ''}
      </div>
    {/if}
  </div>
</section>
