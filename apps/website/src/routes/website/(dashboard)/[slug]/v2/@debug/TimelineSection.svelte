<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import dayjs from 'dayjs';
  import { eventCategory } from './types';
  import type { DebugBus } from './debug-bus.svelte';
  import type { TimelineEntry } from './types';

  type Props = {
    bus: DebugBus;
    onSelectHash?: (hash: string) => void;
  };

  let { bus, onSelectHash }: Props = $props();

  const categoryStyle = {
    commit: css.raw({ color: 'palette.blue' }),
    push: css.raw({ color: 'palette.green' }),
    subscription: css.raw({ color: 'palette.purple' }),
  } as const;

  const reversed = $derived(bus.entries.toReversed());

  const fmtAbsTime = (ts: number): string => dayjs(ts).format('HH:mm:ss.SSS');

  const summary = (e: TimelineEntry): { hash: string | null; rest: string } => {
    switch (e.kind) {
      case 'commit.created': {
        return { hash: e.hash, rest: ` · chain=${e.chainSize}` };
      }
      case 'push.fired': {
        return { hash: null, rest: `${e.commits}c/${e.objects}o` };
      }
      case 'push.success': {
        return { hash: null, rest: `${e.durationMs.toFixed(0)}ms` };
      }
      case 'push.error': {
        return { hash: null, rest: e.message };
      }
      case 'subscription.received': {
        return { hash: e.newHead, rest: ` (${e.ownEcho ? 'own' : 'foreign'}) · ${e.commits}c/${e.objects}o` };
      }
    }
  };
</script>

<section
  class={css({
    paddingX: '12px',
    paddingY: '8px',
    borderBottomWidth: '1px',
    borderBottomColor: 'border.subtle',
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
    <span>
      EVENTS <span class={css({ color: 'text.faint', fontWeight: 'normal' })}>· {bus.entries.length}</span>
    </span>
  </header>
  <ul
    class={css({
      fontFamily: 'mono',
      fontSize: '10px',
      lineHeight: '[1.55]',
      height: '200px',
      overflowY: 'auto',
      listStyle: 'none',
      paddingLeft: '0',
    })}
  >
    {#each reversed as entry (entry.id)}
      {@const cat = eventCategory(entry.kind)}
      {@const sum = summary(entry)}
      <li
        class={css({
          display: 'flex',
          gap: '8px',
          paddingY: '1px',
        })}
      >
        <span class={css({ color: 'text.faint', flexGrow: '0', flexShrink: '0' })}>{fmtAbsTime(entry.ts)}</span>
        <span class={css({ flexGrow: '0', flexShrink: '0' }, categoryStyle[cat])}>{entry.kind}</span>
        <span class={css({ flexGrow: '1', flexShrink: '1', minWidth: '0' })}>
          {#if sum.hash}
            {@const hash = sum.hash}
            <button
              class={css({
                fontFamily: 'mono',
                textAlign: 'left',
                cursor: 'pointer',
                backgroundColor: 'transparent',
                border: 'none',
                padding: '0',
                _hover: { textDecoration: 'underline' },
              })}
              onclick={() => onSelectHash?.(hash)}
              title={hash}
              type="button"
            >
              {hash.slice(0, 8)}
            </button>
          {/if}{sum.rest}
        </span>
      </li>
    {/each}
  </ul>
</section>
