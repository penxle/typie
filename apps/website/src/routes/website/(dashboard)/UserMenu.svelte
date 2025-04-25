<script lang="ts">
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

  <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.500', letterSpacing: '[0]' })}>{$user.email}</div>

  <div class={css({ marginTop: '16px', width: 'full', height: '1px', backgroundColor: 'gray.200' })}></div>

  <button
    class={css({
      paddingX: '16px',
      paddingY: '8px',
      width: 'full',
      fontSize: '14px',
      fontWeight: 'medium',
      color: 'gray.700',
      transition: 'common',
      _hover: { backgroundColor: 'gray.100' },
    })}
    onclick={() => {
      pushState('', { shallowRoute: '/preference/account' });
      open = false;
    }}
    type="button"
  >
    설정
  </button>

  <div class={css({ width: 'full', height: '1px', backgroundColor: 'gray.200' })}></div>

  <button
    class={css({
      paddingX: '16px',
      paddingY: '8px',
      width: 'full',
      fontSize: '14px',
      fontWeight: 'medium',
      color: 'red.500',
      transition: 'common',
      _hover: { backgroundColor: 'red.50' },
    })}
    onclick={() => {
      location.href = qs.stringifyUrl({
        url: `${env.PUBLIC_AUTH_URL}/logout`,
        query: {
          redirect_uri: env.PUBLIC_WEBSITE_URL,
        },
      });
    }}
    type="button"
  >
    로그아웃
  </button>
</Modal>
