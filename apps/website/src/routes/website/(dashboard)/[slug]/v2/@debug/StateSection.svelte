<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { DebugSnapshot } from './types';

  type Props = {
    snapshot: DebugSnapshot;
    onSelectHash?: (hash: string) => void;
  };

  let { snapshot, onSelectHash }: Props = $props();

  const trunc = (h: string) => (h ? h.slice(0, 8) : '—');

  const ahead = $derived(snapshot.chainTip && snapshot.chainTip !== snapshot.serverHeadHash ? snapshot.outbox.length : 0);

  const lost = $derived(
    Boolean(
      snapshot.chainTip && snapshot.serverHeadHash && snapshot.chainTip !== snapshot.serverHeadHash && snapshot.pushStatus === 'error',
    ),
  );
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
    <span>STATE</span>
  </header>
  <div class={css({ display: 'grid', gridTemplateColumns: 'auto 1fr', columnGap: '12px', rowGap: '2px' })}>
    <span class={css({ color: 'text.muted' })}>status</span>
    <span class={css({ display: 'flex', alignItems: 'center', gap: '6px' })}>
      <span
        class={css({
          display: 'inline-block',
          width: '6px',
          height: '6px',
          borderRadius: 'full',
          backgroundColor:
            snapshot.pushStatus === 'idle' ? 'palette.green' : snapshot.pushStatus === 'pushing' ? 'palette.orange' : 'palette.red',
        })}
      ></span>
      {snapshot.pushStatus}
    </span>

    <span class={css({ color: 'text.muted' })}>server head</span>
    {#if snapshot.serverHeadHash}
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
        onclick={() => onSelectHash?.(snapshot.serverHeadHash)}
        title={snapshot.serverHeadHash}
        type="button"
      >
        {trunc(snapshot.serverHeadHash)}
      </button>
    {:else}
      <span class={css({ fontFamily: 'mono' })}>—</span>
    {/if}

    <span class={css({ color: 'text.muted' })}>local head</span>
    {#if snapshot.chainTip}
      <button
        class={css({
          fontFamily: 'mono',
          textAlign: 'left',
          cursor: 'pointer',
          backgroundColor: 'transparent',
          border: 'none',
          padding: '0',
          color: ahead > 0 ? 'palette.blue' : lost ? 'palette.red' : '[inherit]',
          _hover: { textDecoration: 'underline' },
        })}
        onclick={() => onSelectHash?.(snapshot.chainTip)}
        title={snapshot.chainTip}
        type="button"
      >
        {trunc(snapshot.chainTip)}
        {#if ahead > 0}
          <span class={css({ color: 'text.faint' })}>(+{ahead} ahead)</span>
        {/if}
        {#if lost}
          <span class={css({ color: 'palette.red' })}>(lost)</span>
        {/if}
      </button>
    {:else}
      <span class={css({ fontFamily: 'mono' })}>—</span>
    {/if}
  </div>
</section>
