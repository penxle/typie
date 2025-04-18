<script lang="ts">
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import { goto } from '$app/navigation';
  import { fragment, graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Icon, Img } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import SettingModal from './SettingModal.svelte';
  import type { DashboardLayout_UserMenu_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_UserMenu_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_UserMenu_user on User {
        id
        name
        email

        ...DashboardLayout_SettingModal_user

        avatar {
          id
          ...Img_image
        }

        plan {
          id

          plan {
            id
            name
          }
        }
      }
    `),
  );

  const logout = graphql(`
    mutation DashboardLayout_UserMenu_Logout_Mutation {
      logout
    }
  `);

  let open = $state(false);
  let clientWidth = $state(0);
  let settingModalOpen = $state(false);

  const app = getAppContext();

  const { anchor, floating } = createFloatingActions({
    placement: 'top',
    offset: 4,
    onClickOutside: () => {
      open = false;
    },
  });

  $effect(() => {
    if (!app.state.sidebarTriggered) {
      open = false;
    }
  });
</script>

<button
  class={flex({
    alignItems: 'center',
    justifyContent: 'space-between',
    marginX: '8px',
    marginTop: '6px',
    marginBottom: '10px',
    borderRadius: '6px',
    paddingX: '8px',
    paddingY: '6px',
    textAlign: 'left',
    transitionProperty: 'background-color',
    transitionDuration: '200ms',
    transitionTimingFunction: 'ease',
    _hover: { backgroundColor: 'gray.100' },
  })}
  onclick={() => (open = true)}
  type="button"
  bind:clientWidth
  use:anchor
>
  <div class={flex({ align: 'center', gap: '8px' })}>
    <Img style={css.raw({ size: '32px', borderRadius: '6px' })} $image={$user.avatar} alt={`${$user.name}의 아바타`} size={32} />

    <div class={flex({ flexDirection: 'column' })}>
      <div class={css({ fontSize: '13px', fontWeight: 'medium' })}>
        {$user.name}
      </div>

      <div class={css({ fontSize: '12px', color: 'gray.500' })}>
        {$user.plan?.plan.name ?? '무료'} 플랜 이용중
      </div>
    </div>
  </div>

  <div class={center({ size: '24px', color: 'gray.500', _hover: { color: 'gray.700', backgroundColor: 'gray.100' } })}>
    <Icon icon={ChevronDownIcon} size={16} />
  </div>
</button>

{#if open}
  <div
    style:width={`${clientWidth}px`}
    class={flex({
      flexDirection: 'column',
      gap: '4px',
      borderWidth: '1px',
      borderRadius: '6px',
      padding: '4px',
      fontSize: '13px',
      color: 'gray.700',
      backgroundColor: 'white',
      zIndex: '50',
    })}
    use:floating
  >
    <div class={flex({ flexDirection: 'column', gap: '4px', paddingX: '8px', paddingY: '4px' })}>
      <div class={css({ color: 'gray.500', fontWeight: 'medium', letterSpacing: '[0]' })}>{$user.email}</div>
    </div>

    <div class={css({ marginX: '4px', height: '1px', backgroundColor: 'gray.100' })}></div>

    <button
      class={css({ paddingX: '8px', paddingY: '4px', textAlign: 'left', _hover: { backgroundColor: 'gray.100' } })}
      onclick={() => {
        open = false;
        settingModalOpen = true;
      }}
      type="button"
    >
      설정
    </button>

    <a
      class={flex({
        alignItems: 'center',
        justifyContent: 'space-between',
        paddingX: '8px',
        paddingY: '4px',
        textAlign: 'left',
        _hover: { backgroundColor: 'gray.100' },
      })}
      href="https://help.typie.co"
      rel="noopener noreferrer"
      target="_blank"
    >
      <span>도움말</span>
      <Icon style={css.raw({ color: 'gray.500' })} icon={ExternalLinkIcon} size={14} />
    </a>

    <div class={css({ marginX: '4px', height: '1px', backgroundColor: 'gray.100' })}></div>

    <button
      class={css({ paddingX: '8px', paddingY: '4px', textAlign: 'left', _hover: { backgroundColor: 'gray.100' } })}
      onclick={async () => {
        await logout();
        await goto('/');
      }}
      type="button"
    >
      로그아웃
    </button>
  </div>
{/if}

<SettingModal {$user} bind:open={settingModalOpen} />
