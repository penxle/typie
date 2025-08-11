<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import { serializeOAuthState } from '@typie/ui/utils';
  import qs from 'query-string';
  import GlobeIcon from '~icons/lucide/globe';
  import Logo from '$assets/logos/logo.svg?component';
  import { PUBLIC_AUTH_URL, PUBLIC_OIDC_CLIENT_ID } from '$env/static/public';
</script>

<main class={center({ height: 'full' })}>
  <div class={flex({ flexDirection: 'column', gap: '24px' })}>
    <Logo class={css({ height: '40px' })} />

    <p class={css({ fontSize: '15px', textAlign: 'center', lineHeight: '[1.6]' })}>
      <span>작성, 정리, 공유까지.</span>
      <br />
      <span class={css({ fontWeight: 'bold' })}>
        글쓰기의 모든 과정을
        <br />
        타이피 하나로 해결해요.
      </span>
    </p>

    <Button
      onclick={async () => {
        const url = qs.stringifyUrl({
          url: `${PUBLIC_AUTH_URL}/authorize`,
          query: {
            client_id: PUBLIC_OIDC_CLIENT_ID,
            response_type: 'code',
            redirect_uri: `${PUBLIC_AUTH_URL}/desktop`,
            state: serializeOAuthState({ redirect_uri: 'typie:///auth/callback' }),
          },
        });

        await openUrl(url);
      }}
    >
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon icon={GlobeIcon} />
        <div class={css({ lineHeight: '[1]' })}>브라우저로 로그인</div>
      </div>
    </Button>
  </div>
</main>
