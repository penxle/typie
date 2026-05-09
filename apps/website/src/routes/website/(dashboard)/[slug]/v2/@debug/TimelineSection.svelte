<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import dayjs from 'dayjs';
  import { eventCategory } from './types';
  import type { DebugBus } from './debug-bus.svelte';
  import type { DebugEventCategory, TimelineEntry } from './types';

  type Props = {
    bus: DebugBus;
  };

  let { bus }: Props = $props();

  const colorStyles = {
    push: css.raw({ color: 'palette.green' }),
    subscription: css.raw({ color: 'palette.blue' }),
    poll: css.raw({ color: 'palette.yellow' }),
  } as const satisfies Record<DebugEventCategory, unknown>;

  function fmtTime(ts: number): string {
    return dayjs(ts).format('HH:mm:ss.SSS');
  }

  function describe(entry: TimelineEntry): string {
    switch (entry.kind) {
      case 'push.fired': {
        return `push.fired (${entry.bytes}B)`;
      }
      case 'push.success': {
        return `push.success (${entry.durationMs.toFixed(0)}ms)`;
      }
      case 'push.error': {
        return `push.error: ${entry.message}`;
      }
      case 'subscription.received': {
        return `subscription.received (${entry.bytes}B)`;
      }
      case 'poll.applied': {
        return `poll.applied (${entry.bytes}B)`;
      }
    }
  }

  const reversed = $derived([...bus.entries].toReversed());
</script>

<section
  class={css({
    flexGrow: '1',
    flexShrink: '1',
    minHeight: '0',
    paddingX: '12px',
    paddingY: '8px',
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
    <span>TIMELINE</span>
    <button
      class={css({ fontSize: '10px', color: 'text.faint', cursor: 'pointer', backgroundColor: 'transparent', border: 'none' })}
      onclick={() => bus.clear()}
      type="button"
    >
      clear
    </button>
  </header>

  <ul
    class={css({
      flexGrow: '1',
      overflowY: 'auto',
      listStyle: 'none',
      paddingLeft: '0',
      display: 'flex',
      flexDirection: 'column',
      gap: '2px',
    })}
  >
    {#each reversed as entry (entry.id)}
      <li class={css({ display: 'flex', gap: '8px', fontFamily: 'mono', fontSize: '10px' })}>
        <span class={css({ color: 'text.faint' })}>{fmtTime(entry.ts)}</span>
        <span class={css(colorStyles[eventCategory(entry.kind)])}>●</span>
        <span class={css({ color: 'text.default', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>
          {describe(entry)}
        </span>
      </li>
    {/each}
  </ul>
</section>
