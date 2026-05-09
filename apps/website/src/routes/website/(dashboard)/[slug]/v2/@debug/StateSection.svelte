<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { DebugSnapshot } from './types';

  type Props = {
    snapshot: DebugSnapshot;
  };

  let { snapshot }: Props = $props();

  const rows = $derived([
    { label: 'editor', value: snapshot.hasEditor ? 'mounted' : '—' },
    { label: 'push status', value: snapshot.pushStatus },
    { label: 'retry attempt', value: String(snapshot.retryAttempt) },
    { label: 'lastSent heads (bytes)', value: String(snapshot.lastSentHeadsBytes) },
  ]);
</script>

<section
  class={css({
    flexShrink: '0',
    paddingX: '12px',
    paddingY: '8px',
    borderBottomWidth: '1px',
    borderBottomColor: 'border.subtle',
  })}
>
  <header
    class={css({
      fontWeight: 'semibold',
      fontSize: '10px',
      letterSpacing: '0.04em',
      color: 'text.muted',
      marginBottom: '6px',
    })}
  >
    STATE
  </header>

  <dl class={css({ display: 'grid', gridTemplateColumns: '[max-content 1fr]', columnGap: '12px', rowGap: '4px' })}>
    {#each rows as row (row.label)}
      <dt class={css({ fontSize: '10px', color: 'text.faint' })}>{row.label}</dt>
      <dd class={css({ fontFamily: 'mono', fontSize: '10px', color: 'text.default' })}>{row.value}</dd>
    {/each}
  </dl>
</section>
