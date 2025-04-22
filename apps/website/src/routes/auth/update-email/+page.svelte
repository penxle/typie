<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { graphql } from '$graphql';
  import { Helmet, RingSpinner } from '$lib/components';
  import { Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  const updateEmail = graphql(`
    mutation UpdateEmailPage_UpdateEmail_Mutation($input: UpdateEmailInput!) {
      updateEmail(input: $input)
    }
  `);

  onMount(async () => {
    await updateEmail({
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      code: page.url.searchParams.get('code')!,
    });

    Toast.success('이메일이 변경되었어요');

    await goto('/', { replaceState: true });
  });
</script>

<Helmet title="이메일 변경 중..." />

<div class={flex({ flexDirection: 'column', gap: '24px' })}>
  <div class={flex({ justifyContent: 'flex-start' })}>
    <Logo class={css({ height: '20px' })} />
  </div>

  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <h1 class={css({ fontSize: '24px', fontWeight: 'extrabold' })}>이메일 변경 중...</h1>
    <div class={css({ fontSize: '14px', color: 'gray.500' })}>잠시만 기다려주세요.</div>
  </div>

  <div class={center({ height: '100px' })}>
    <RingSpinner style={css.raw({ size: '50px', color: 'brand.500' })} />
  </div>
</div>
