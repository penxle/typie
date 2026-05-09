<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import dayjs from 'dayjs';

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
    onSelectCommit?: (id: string) => void;
  };

  let { onSelectCommit }: Props = $props();

  // Data source intentionally disabled in this cycle.
  // Lane DAG layout algorithm and visual styling are preserved verbatim;
  // wire-up to a real changeset/op DAG source is a follow-up cycle.
  const commits: CommitLite[] = [];
  const localChainReversed: { commitHash: string; committedAt: string }[] = [];

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

  function fmtTime(iso: string): string {
    return dayjs(iso).format('HH:mm:ss');
  }
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
      alignItems: 'center',
      fontWeight: 'semibold',
      fontSize: '10px',
      letterSpacing: '0.04em',
      color: 'text.muted',
      marginBottom: '6px',
    })}
  >
    <span>GRAPH</span>
    <span class={css({ fontSize: '10px', color: 'text.faint', fontWeight: 'normal' })}>data source pending</span>
  </header>

  <div class={css({ flexGrow: '1', flexShrink: '1', minHeight: '0', overflowY: 'auto' })}>
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
  </div>
</section>
