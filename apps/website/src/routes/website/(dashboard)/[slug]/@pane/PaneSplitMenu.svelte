<script lang="ts">
  import { center } from '@typie/styled-system/patterns';
  import { Icon, Menu, MenuItem } from '@typie/ui/components';
  import mixpanel from 'mixpanel-browser';
  import Columns2Icon from '~icons/lucide/columns-2';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import Rows2Icon from '~icons/lucide/rows-2';
  import { getPane, getPaneGroup } from './context.svelte';

  const pane = getPane();
  const paneGroup = getPaneGroup();

  const add = (direction: 'horizontal' | 'vertical') => {
    const init = pane.kind === 'home' ? ({ kind: 'home' } as const) : ({ kind: 'entity', slug: pane.slug } as const);
    paneGroup.addPane(init, { paneId: pane.id, side: direction === 'horizontal' ? 'right' : 'bottom' });
    mixpanel.track('add_pane', { via: 'pane_header', direction });
  };
</script>

<Menu placement="bottom-end">
  {#snippet button({ open })}
    <button
      class={center({
        borderRadius: '4px',
        size: '24px',
        color: 'text.muted',
        transition: 'common',
        _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
        _pressed: { color: 'text.default', backgroundColor: 'surface.active' },
      })}
      aria-label="창 메뉴"
      aria-pressed={open}
      type="button"
    >
      <Icon icon={EllipsisIcon} size={16} />
    </button>
  {/snippet}

  <MenuItem icon={Columns2Icon} onclick={() => add('horizontal')}>오른쪽에 열기</MenuItem>
  <MenuItem icon={Rows2Icon} onclick={() => add('vertical')}>아래에 열기</MenuItem>
</Menu>
