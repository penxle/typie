<script lang="ts">
  import qs from 'query-string';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { env } from '$env/dynamic/public';
  import { fragment, graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Button, Img } from '$lib/components';
  import { serializeOAuthState } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Optional, UsersiteWildcardSlugPage_Header_user } from '$graphql';

  type Props = {
    $user: Optional<UsersiteWildcardSlugPage_Header_user>;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment UsersiteWildcardSlugPage_Header_user on User {
        id
        name
        email

        avatar {
          id
          ...Img_image
        }
      }
    `),
  );

  let open = $state(false);

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom-end',
    offset: 4,
    onClickOutside: () => {
      open = false;
    },
  });

  const authorizeUrl = $derived(
    qs.stringifyUrl({
      url: `${env.PUBLIC_AUTH_URL}/authorize`,
      query: {
        client_id: env.PUBLIC_OIDC_CLIENT_ID,
        response_type: 'code',
        redirect_uri: `${page.url.origin}/authorize`,
        state: serializeOAuthState({ redirect_uri: page.url.href }),
      },
    }),
  );
</script>

<div
  class={flex({
    align: 'center',
    justify: 'space-between',
    position: 'sticky',
    top: '0',
    borderBottomWidth: '1px',
    borderBottomColor: 'gray.200',
    paddingX: '20px',
    height: '52px',
    backgroundColor: 'white',
    zIndex: '50',
  })}
>
  <Logo class={css({ flexShrink: '0', height: '20px' })} />

  {#if $user}
    <button onclick={() => (open = true)} type="button" use:anchor>
      <Img
        style={css.raw({ size: '32px', borderWidth: '1px', borderColor: 'gray.100', borderRadius: '6px' })}
        $image={$user.avatar}
        alt={`${$user.name}의 아바타`}
        size={32}
      />
    </button>
  {:else}
    <Button external href={authorizeUrl} size="sm" type="link" variant="primary">시작하기</Button>
  {/if}
</div>

{#if open}
  <div
    class={flex({
      flexDirection: 'column',
      gap: '4px',
      borderWidth: '1px',
      borderRadius: '6px',
      borderColor: 'gray.200',
      padding: '4px',
      fontSize: '13px',
      color: 'gray.700',
      backgroundColor: 'white',
      width: '160px',
      boxShadow: 'small',
      zIndex: '50',
    })}
    use:floating
  >
    <a
      class={css({ borderRadius: '3px', paddingX: '8px', paddingY: '4px', textAlign: 'left', _hover: { backgroundColor: 'gray.100' } })}
      href={`${env.PUBLIC_WEBSITE_URL}/home`}
      rel="noopener noreferrer"
      target="_blank"
    >
      내 홈으로
    </a>
    <button
      class={css({ borderRadius: '3px', paddingX: '8px', paddingY: '4px', textAlign: 'left', _hover: { backgroundColor: 'gray.100' } })}
      onclick={() => {
        location.href = qs.stringifyUrl({
          url: `${env.PUBLIC_AUTH_URL}/logout`,
          query: {
            redirect_uri: page.url.href,
          },
        });
      }}
      type="button"
    >
      로그아웃
    </button>
  </div>
{/if}
