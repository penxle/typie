<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, HorizontalDivider, Icon, Menu } from '@typie/ui/components';
  import HouseIcon from '~icons/lucide/house';
  import LogOutIcon from '~icons/lucide/log-out';
  import { env } from '$env/dynamic/public';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import ThemeSegment from './ThemeSegment.svelte';
  import type { UsersiteHeader_user$key } from '$mearie';

  type Props = {
    user$key: UsersiteHeader_user$key | null | undefined;
    authorizeUrl: string;
    onLogout: () => void;
    open?: boolean;
  };

  let { user$key, authorizeUrl, onLogout, open = $bindable(false) }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment UsersiteHeader_user on User {
        id
        name

        avatar {
          id
          ...Img_image
        }
      }
    `),
    () => user$key,
  );

  const menuRow = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    paddingX: '8px',
    paddingY: '6px',
    borderRadius: '6px',
    fontSize: '13px',
    fontWeight: 'medium',
    color: 'text.default',
    textAlign: 'left',
    outlineWidth: '0',
    transition: 'common',
    cursor: 'pointer',
    _hover: { backgroundColor: 'surface.hover' },
    _focus: { backgroundColor: 'surface.hover' },
  });
</script>

{#if user.data}
  <Menu
    style={css.raw({
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      size: '32px',
      padding: '2px',
      marginRight: '-2px',
      borderRadius: 'full',
      transition: '[opacity 150ms ease-out]',
      _hover: { opacity: '[0.8]' },
      _expanded: { opacity: '[0.8]' },
    })}
    listStyle={css.raw({
      gap: '0',
      minWidth: '200px',
      padding: '4px',
      borderWidth: '1px',
      borderColor: 'border.default',
      boxShadow: 'md',
    })}
    offset={6}
    placement="bottom-end"
    bind:open
  >
    {#snippet button()}
      {#if user.data?.avatar}
        <Img
          style={css.raw({ size: '28px', borderRadius: 'full', boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]' })}
          alt={`${user.data.name}의 아바타`}
          image$key={user.data.avatar}
          size={64}
        />
      {:else}
        <div class={css({ size: '28px', borderRadius: 'full', backgroundColor: 'accent.subtle' })}></div>
      {/if}
    {/snippet}

    <div class={flex({ alignItems: 'center', gap: '8px', paddingX: '8px', paddingY: '6px' })} role="none">
      {#if user.data.avatar}
        <Img style={css.raw({ flexShrink: '0', size: '24px', borderRadius: 'full' })} alt="" image$key={user.data.avatar} size={32} />
      {:else}
        <div class={css({ flexShrink: '0', size: '24px', borderRadius: 'full', backgroundColor: 'accent.subtle' })}></div>
      {/if}
      <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default', truncate: true })}>{user.data.name}</span>
    </div>

    <HorizontalDivider style={css.raw({ marginY: '4px' })} color="secondary" />

    <a class={css(menuRow)} href={env.PUBLIC_WEBSITE_URL} role="menuitem" tabindex="-1">
      <Icon style={css.raw({ flexShrink: '0', color: 'text.default' })} icon={HouseIcon} size={14} />
      <span>내 홈으로</span>
    </a>

    <ThemeSegment via="header" />

    <HorizontalDivider style={css.raw({ marginY: '4px' })} color="secondary" />

    <button
      class={css(menuRow, { color: 'danger.default' })}
      onclick={() => {
        open = false;
        onLogout();
      }}
      role="menuitem"
      tabindex="-1"
      type="button"
    >
      <Icon style={css.raw({ flexShrink: '0' })} icon={LogOutIcon} size={14} />
      <span>로그아웃</span>
    </button>
  </Menu>
{:else}
  <Button external href={authorizeUrl} size="sm" type="link" variant="primary">시작하기</Button>
{/if}
