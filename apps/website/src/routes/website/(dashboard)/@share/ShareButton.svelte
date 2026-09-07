<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { EntityVisibility } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, VerticalDivider } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import BlendIcon from '~icons/lucide/blend';
  import ClockIcon from '~icons/lucide/clock';
  import GlobeIcon from '~icons/lucide/globe';
  import LinkIcon from '~icons/lucide/link';
  import { graphql } from '$mearie';
  import type { DashboardLayout_Share_ShareButton_document$key } from '$mearie';

  type Props = {
    document$key: DashboardLayout_Share_ShareButton_document$key;
  };

  let { document$key }: Props = $props();

  const app = getAppContext();

  const document = createFragment(
    graphql(`
      fragment DashboardLayout_Share_ShareButton_document on Document {
        id

        entity {
          id
          visibility
        }

        publication {
          id
          state
        }
      }
    `),
    () => document$key,
  );

  const publishedStyle = css.raw({ color: 'palette.green' });
  const scheduledStyle = css.raw({ color: 'palette.orange' });
  const unlistedStyle = css.raw({ color: 'palette.purple' });
  const privateStyle = css.raw({ color: 'palette.gray' });

  const state = $derived(
    document.data.publication?.state === 'PUBLISHED'
      ? { icon: GlobeIcon, label: '발행됨', style: publishedStyle }
      : document.data.publication?.state === 'SCHEDULED'
        ? { icon: ClockIcon, label: '발행 예약됨', style: scheduledStyle }
        : document.data.entity.visibility === EntityVisibility.UNLISTED
          ? { icon: LinkIcon, label: '링크 공개 중', style: unlistedStyle }
          : { icon: BlendIcon, label: '공유', style: privateStyle },
  );
</script>

<button
  class={flex({
    alignItems: 'center',
    gap: '6px',
    height: '24px',
    paddingX: '8px',
    borderRadius: '4px',
    fontSize: '12px',
    fontWeight: 'medium',
    whiteSpace: 'nowrap',
    color: 'text.default',
    transition: 'common',
    _hover: { backgroundColor: 'surface.hover' },
  })}
  onclick={() => (app.state.shareOpen = [document.data.entity.id])}
  type="button"
>
  <Icon style={state.style} icon={state.icon} size={16} />
  <span>{state.label}</span>
</button>

<VerticalDivider style={css.raw({ height: '12px' })} />
