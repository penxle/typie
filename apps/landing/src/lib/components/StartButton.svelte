<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import { siteUrl } from '$lib/site';
  import CtaLink from './CtaLink.svelte';

  type Props = { badge?: string };

  let { badge }: Props = $props();

  const badgeClass = css({
    position: 'absolute',
    top: '[-10px]',
    right: '[-12px]',
    paddingX: '8px',
    paddingY: '3px',
    borderRadius: 'full',
    backgroundColor: 'success.default',
    color: 'text.on.success',
    fontFamily: 'ui',
    fontSize: '11px',
    fontWeight: 'semibold',
    lineHeight: '[1.2]',
    whiteSpace: 'nowrap',
    pointerEvents: 'none',
  });
</script>

<span class={css({ position: 'relative', display: 'inline-block' })}>
  <CtaLink href={siteUrl('/start')}>
    <span>시작하기</span>
    <Icon
      style={css.raw({ transition: '[transform 0.2s ease-out]', _groupHover: { transform: 'translateX(2px)' } })}
      icon={ArrowRightIcon}
      size={16}
    />
  </CtaLink>
  {#if badge}
    <span class={badgeClass}>{badge}</span>
  {/if}
</span>
