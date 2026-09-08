<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import type { Platform } from '$lib/platform';

  type Props = { platform: Platform };

  let { platform }: Props = $props();

  const LABEL = '다운로드';

  const buttonClass = css({
    display: 'inline-flex',
    alignItems: 'center',
    gap: '10px',
    height: '52px',
    paddingX: '20px',
    borderWidth: '1px',
    borderColor: 'border.emphasis',
    borderRadius: '8px',
    fontFamily: 'ui',
    fontSize: '16px',
    fontWeight: 'semibold',
    letterSpacing: '-0.01em',
    color: 'text.default',
    transition: '[background-color 0.2s ease-out]',
    _hover: { backgroundColor: 'surface.hover' },
  });
  const noteClass = css({ marginTop: '1px', fontSize: '11px', fontWeight: 'normal', letterSpacing: '[0]', color: 'text.hint' });
</script>

<a class={buttonClass} href={platform.url} rel={platform.external ? 'noopener noreferrer' : undefined}>
  <Icon icon={platform.icon} size={16} />
  {#if platform.note}
    <span class={flex({ direction: 'column', align: 'flex-start' })}>
      <span>{LABEL}</span>
      <span class={noteClass}>{platform.note}</span>
    </span>
  {:else}
    <span>{LABEL}</span>
  {/if}
</a>
