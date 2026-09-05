<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { scrollFog } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import { expand } from './lib/motion.ts';
  import type { ToolRow } from './lib/tool-calls.ts';

  type Props = { count: number; rows: ToolRow[] };

  let { count, rows }: Props = $props();

  let open = $state(false);

  const toggleClass = css({ display: 'inline-flex', alignItems: 'center', gap: '4px', fontSize: '12px', color: 'text.hint' });
  const chevronStyle = css.raw({ transition: '[transform 150ms cubic-bezier(0.23, 1, 0.32, 1)]', _motionReduce: { transition: '[none]' } });
  const chevronOpenStyle = css.raw({ transform: 'rotate(180deg)' });
  const listWrapClass = css({ marginTop: '6px' });
  const listClass = css({
    paddingLeft: '10px',
    borderLeftWidth: '2px',
    borderColor: 'border.hairline',
    maxHeight: '168px',
    overflowY: 'auto',
    scrollbarWidth: 'none',
  });
  const rowClass = css({
    fontSize: '12px',
    lineHeight: '[1.75]',
    color: 'text.muted',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  });
</script>

{#if count > 0}
  <div>
    <button class={toggleClass} aria-expanded={open} onclick={() => (open = !open)} type="button">
      도구 {count}회 호출함
      <Icon style={css.raw(chevronStyle, open ? chevronOpenStyle : {})} icon={ChevronDownIcon} size={10} />
    </button>
    {#if open}
      <div class={listWrapClass} transition:expand>
        <div class={listClass} use:scrollFog={{ orientation: 'vertical', size: 16 }}>
          {#each rows as entry, index (index)}
            <div class={rowClass}>
              {entry.label}{#if entry.count > 1}<span class={css({ marginLeft: '4px', color: 'text.hint' })}>×{entry.count}</span>{/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>
{/if}
