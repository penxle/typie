<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import ArrowUpRightIcon from '~icons/lucide/arrow-up-right';
  import DownloadIcon from '~icons/lucide/download';
  import CtaLink from '$lib/components/CtaLink.svelte';
  import type { Primary } from './download';

  type Props = { primary: Primary };

  let { primary }: Props = $props();

  const noteClass = css({ marginTop: '12px', fontFamily: 'ui', fontSize: '13px', color: 'text.hint' });
</script>

<div>
  <CtaLink external={primary.item.external} href={primary.item.url} iconSide="leading">
    <Icon icon={primary.item.external ? ArrowUpRightIcon : DownloadIcon} size={18} />
    {primary.label}
  </CtaLink>
  <p class={noteClass}>{primary.note}</p>
</div>
