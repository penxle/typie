<script lang="ts">
  import { message } from '@tauri-apps/plugin-dialog';
  import { fetch } from '@tauri-apps/plugin-http';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { RingSpinner } from '@typie/ui/components';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { PUBLIC_AUTH_URL, PUBLIC_OIDC_CLIENT_ID, PUBLIC_OIDC_CLIENT_SECRET } from '$env/static/public';

  const login = async () => {
    try {
      const code = page.url.searchParams.get('code');

      if (!code) {
        return;
      }

      const response = await fetch(`${PUBLIC_AUTH_URL}/token`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
        },
        body: new URLSearchParams({
          code,
          grant_type: 'authorization_code',
          redirect_uri: `${PUBLIC_AUTH_URL}/desktop`,
          client_id: PUBLIC_OIDC_CLIENT_ID,
          client_secret: PUBLIC_OIDC_CLIENT_SECRET,
        }),
      });

      const data = await response.json();
      if (data.error) {
        message(data.error_description);
        await goto('/auth/login');
      }

      message(data.access_token);
      await goto('/auth/login'); // TODO: 로그인 토큰 저장하기
    } catch (err) {
      message(String(err));
      await goto('/auth/login');
    }
  };

  onMount(() => {
    login();
  });
</script>

<main class={center({ height: 'full' })}>
  <div class={flex({ flexDirection: 'column', gap: '24px', alignItems: 'center' })}>
    <RingSpinner style={css.raw({ size: '40px', color: 'accent.brand.default' })} />
    <div class={css({ fontSize: '15px', fontWeight: 'medium', textAlign: 'center', lineHeight: '[1.6]' })}>
      작업 공간을 준비하고 있어요.
      <br />
      잠시만 기다려주세요!
    </div>
  </div>
</main>
