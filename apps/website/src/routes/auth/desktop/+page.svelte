<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Helmet } from '@typie/ui/components';
  import { deserializeOAuthState } from '@typie/ui/utils';
  import qs from 'query-string';
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';

  const openApp = () => {
    const code = page.url.searchParams.get('code');
    const state = page.url.searchParams.get('state');

    if (!code || !state) {
      return;
    }

    const { redirect_uri } = deserializeOAuthState(state);
    const url = qs.stringifyUrl({
      url: redirect_uri,
      query: {
        code,
      },
    });

    window.open(url, '_blank');
  };

  onMount(() => {
    openApp();
  });
</script>

<Helmet title="로그인 완료" />

<div class={flex({ flexDirection: 'column', gap: '24px' })}>
  <div class={flex({ justifyContent: 'flex-start' })}>
    <Logo class={css({ height: '32px' })} />
  </div>

  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <h1 class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold' })}>로그인 완료</h1>
    <div class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'text.faint' })}>앱이 자동으로 열려요. 이 창은 닫아도 돼요.</div>
  </div>

  <Button onclick={openApp}>앱으로 돌아가기</Button>
</div>
