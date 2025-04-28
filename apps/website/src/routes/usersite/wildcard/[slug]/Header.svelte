<script lang="ts">
  import qs from 'query-string';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { env } from '$env/dynamic/public';
  import { fragment, graphql } from '$graphql';
  import { Button, Img, Menu, MenuItem } from '$lib/components';
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
    justifyContent: 'space-between',
    alignItems: 'center',
    flexShrink: '0',
    borderBottomWidth: '1px',
    borderBottomColor: 'gray.200',
    paddingX: '20px',
    height: '52px',
    backgroundColor: 'white',
  })}
>
  <Logo class={css({ flexShrink: '0', height: '20px' })} />

  {#if $user}
    <Menu>
      {#snippet button()}
        <Img
          style={css.raw({ size: '32px', borderWidth: '1px', borderColor: 'gray.100', borderRadius: 'full' })}
          $image={$user.avatar}
          alt={`${$user.name}의 아바타`}
          size={32}
        />
      {/snippet}

      <MenuItem href={`${env.PUBLIC_WEBSITE_URL}/home`} type="link">내 홈으로</MenuItem>
      <MenuItem
        onclick={() => {
          location.href = qs.stringifyUrl({
            url: `${env.PUBLIC_AUTH_URL}/logout`,
            query: {
              redirect_uri: page.url.href,
            },
          });
        }}
      >
        로그아웃
      </MenuItem>
    </Menu>
  {:else}
    <Button external href={authorizeUrl} size="sm" type="link" variant="primary">시작하기</Button>
  {/if}
</div>
