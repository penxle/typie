<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import qs from 'query-string';
  import { pushState } from '$app/navigation';
  import { env } from '$env/dynamic/public';
  import { fragment, graphql } from '$graphql';
  import { tooltip } from '$lib/actions';
  import { Img, Modal } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
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

        avatar {
          id
          ...Img_image
        }

        subscription {
          id
        }
      }
    `),
  );

  let open = $state(false);
</script>

<button
  class={center({
    borderRadius: 'full',
    size: '32px',
    overflow: 'hidden',
    userSelect: 'none',
    transition: 'common',
    _hover: { transform: 'scale(1.05)' },
  })}
  onclick={() => (open = true)}
  type="button"
  use:tooltip={{ message: '프로필', placement: 'right', offset: 12 }}
>
  <Img style={css.raw({ size: 'full' })} $image={$user.avatar} alt={`${$user.name}의 아바타`} size={32} />
</button>

<Modal style={css.raw({ alignItems: 'center', borderWidth: '0', maxWidth: '300px' })} bind:open>
  <div
    class={css({
      position: 'relative',
      marginBottom: '40px',
      width: 'full',
      height: '100px',
      background: '[linear-gradient(to right, #4776e6, #8e54e9)]',
    })}
  >
    <Img
      style={css.raw({
        position: 'absolute',
        bottom: '0',
        left: '1/2',
        borderWidth: '4px',
        borderColor: 'white',
        borderRadius: 'full',
        translate: 'auto',
        translateX: '-1/2',
        translateY: '1/2',
        size: '80px',
      })}
      $image={$user.avatar}
      alt={`${$user.name}의 아바타`}
      size={128}
    />
  </div>

  <div class={css({ fontSize: '15px', fontWeight: 'semibold', lineClamp: '1' })}>{$user.name}</div>

  <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.faint', letterSpacing: '[0]' })}>{$user.email}</div>

  <div class={css({ marginTop: '16px', width: 'full', height: '1px', backgroundColor: 'interactive.hover' })}></div>

  <button
    class={css({
      paddingX: '16px',
      paddingY: '12px',
      width: 'full',
      fontSize: '14px',
      fontWeight: 'medium',
      color: 'text.subtle',
      transition: 'common',
      _hover: { backgroundColor: 'surface.muted' },
    })}
    onclick={() => {
      pushState('', { shallowRoute: '/preference/account' });
      mixpanel.track('open_preference_modal', { via: 'user_menu' });
      open = false;
    }}
    tabindex={-1}
    type="button"
  >
    설정
  </button>

  <div class={css({ width: 'full', height: '1px', backgroundColor: 'interactive.hover' })}></div>

  {#if $user.subscription}
    <a
      class={css({
        paddingX: '16px',
        paddingY: '12px',
        width: 'full',
        fontSize: '14px',
        fontWeight: 'medium',
        textAlign: 'center',
        color: 'text.subtle',
        transition: 'common',
        _hover: { backgroundColor: 'surface.muted' },
      })}
      href="https://typie.link/community"
      rel="noopener noreferrer"
      target="_blank"
    >
      타이피 커뮤니티
    </a>

    <div class={css({ width: 'full', height: '1px', backgroundColor: 'interactive.hover' })}></div>
  {/if}

  <a
    class={css({
      paddingX: '16px',
      paddingY: '12px',
      width: 'full',
      fontSize: '14px',
      fontWeight: 'medium',
      textAlign: 'center',
      color: 'text.subtle',
      transition: 'common',
      _hover: { backgroundColor: 'surface.muted' },
    })}
    href="https://penxle.channel.io/home"
    rel="noopener noreferrer"
    target="_blank"
  >
    고객센터
  </a>

  <div class={css({ width: 'full', height: '1px', backgroundColor: 'interactive.hover' })}></div>

  <button
    class={css({
      paddingX: '16px',
      paddingY: '12px',
      width: 'full',
      fontSize: '14px',
      fontWeight: 'medium',
      color: 'text.danger',
      transition: 'common',
      _hover: { backgroundColor: 'accent.danger.subtle' },
    })}
    onclick={() => {
      mixpanel.track('logout', { via: 'user_menu' });

      location.href = qs.stringifyUrl({
        url: `${env.PUBLIC_AUTH_URL}/logout`,
        query: {
          redirect_uri: env.PUBLIC_WEBSITE_URL,
        },
      });
    }}
    tabindex={-1}
    type="button"
  >
    로그아웃
  </button>
</Modal>
