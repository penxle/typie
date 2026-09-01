<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { comma } from '@typie/ui/utils';
  import SettingsIcon from '~icons/lucide/settings';
  import XIcon from '~icons/lucide/x';
  import PrismIcon from '~icons/typie/prism';
  import PrismCreditIcon from '~icons/typie/prism-credit';
  import { pushState } from '$app/navigation';
  import type { Snippet } from 'svelte';

  type Props = {
    creditBalance?: number;
    children?: Snippet<[buttonClass: string]>;
  };

  let { creditBalance, children }: Props = $props();

  const app = getAppContext();
  const buttonClass = css({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: '0',
    borderRadius: '4px',
    size: '24px',
    color: 'text.faint',
    transition: '[transform 160ms cubic-bezier(0.23, 1, 0.32, 1)]',
    _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
    _active: { transform: 'scale(0.95)' },
  });
</script>

<header
  class={flex({
    alignItems: 'center',
    gap: '8px',
    height: '44px',
    paddingX: '14px',
    borderBottomWidth: '1px',
    borderColor: 'border.subtle',
    flexShrink: '0',
  })}
>
  {#if creditBalance === undefined}
    <Icon style={css.raw({ color: 'text.subtle', flexShrink: '0' })} icon={PrismIcon} size={16} />
    <span class={css({ fontSize: '13px', fontWeight: 'semibold', letterSpacing: '[0.04em]' })}>PRISM</span>
  {:else}
    <Menu
      style={css.raw({
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        flexShrink: '0',
        marginX: '-4px',
        paddingX: '4px',
        paddingY: '4px',
        borderRadius: '6px',
        transition: 'common',
        _hover: { backgroundColor: 'surface.muted' },
        _expanded: { backgroundColor: 'surface.muted' },
      })}
      buttonAriaLabel="PRISM 메뉴"
      placement="bottom-start"
    >
      {#snippet button()}
        <Icon style={css.raw({ color: 'text.subtle', flexShrink: '0' })} icon={PrismIcon} size={16} />
        <span class={css({ fontSize: '13px', fontWeight: 'semibold', letterSpacing: '[0.04em]' })}>PRISM</span>
      {/snippet}

      <MenuItem icon={SettingsIcon} onclick={() => pushState('', { shallowRoute: '/preference/prism/general' })}>프리즘 설정</MenuItem>
      <MenuItem icon={PrismCreditIcon} onclick={() => pushState('', { shallowRoute: '/preference/prism/credits' })}>
        {#snippet suffix()}
          <span
            class={flex({
              alignItems: 'center',
              gap: '4px',
              marginLeft: 'auto',
              fontSize: '12px',
              color: creditBalance < 0 ? 'text.danger' : 'text.brand',
            })}
          >
            <Icon icon={PrismCreditIcon} size={12} />
            {comma(creditBalance)}
          </span>
        {/snippet}
        크레딧
      </MenuItem>
    </Menu>
  {/if}
  {@render children?.(buttonClass)}
  <button
    class={cx(buttonClass, children ? undefined : css({ marginLeft: 'auto' }))}
    aria-label="PRISM 닫기"
    onclick={() => (app.preference.current.prismPanelOpen = false)}
    type="button"
    use:tooltip={{ message: 'PRISM 닫기', keys: ['Mod', 'E'] }}
  >
    <Icon icon={XIcon} size={16} />
  </button>
</header>
