<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { graphql } from '$graphql';
  import { Button, Helmet, TextInput } from '$lib/components';
  import { createForm, FormError } from '$lib/form';
  import { Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

  const resetPassword = graphql(`
    mutation ResetPasswordPage_ResetPassword_Mutation($input: ResetPasswordInput!) {
      resetPassword(input: $input)
    }
  `);

  const form = createForm({
    schema: z
      .object({
        password: z.string({ required_error: '새 비밀번호를 입력해주세요.' }).nonempty('새 비밀번호를 입력해주세요.'),
        confirmPassword: z.string({ required_error: '비밀번호 확인을 입력해주세요.' }).nonempty('비밀번호 확인을 입력해주세요.'),
      })
      .refine((data) => data.password === data.confirmPassword, {
        path: ['confirmPassword'],
        message: '비밀번호가 일치하지 않습니다.',
      }),
    onSubmit: async (data) => {
      await resetPassword({
        code: page.url.searchParams.get('code') ?? '',
        password: data.password,
      });

      mixpanel.track('reset_password');

      Toast.success('비밀번호가 재설정되었어요');

      await goto('/login', { replaceState: true });
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'invalid_code') {
        throw new FormError('confirmPassword', '만료되었거나 유효하지 않은 링크입니다. 비밀번호 재설정을 다시 요청해주세요.');
      }
    },
  });
</script>

<Helmet title="비밀번호 변경" />

<div class={flex({ flexDirection: 'column', gap: '24px' })}>
  <div class={flex({ justifyContent: 'flex-start' })}>
    <Logo class={css({ height: '32px' })} />
  </div>

  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <h1 class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold' })}>비밀번호를 변경하세요</h1>

    <div class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'gray.500' })}>새로운 비밀번호를 입력해주세요.</div>
  </div>

  <form class={flex({ flexDirection: 'column', gap: '24px' })} onsubmit={form.handleSubmit}>
    <div class={flex({ direction: 'column', gap: '12px' })}>
      <div class={flex({ direction: 'column', gap: '4px' })}>
        <label class={css({ fontSize: '13px', color: 'gray.700', userSelect: 'none' })} for="password">새 비밀번호</label>
        <TextInput
          id="password"
          aria-invalid={!!form.errors.password}
          placeholder="새 비밀번호를 입력하세요"
          type="password"
          bind:value={form.fields.password}
        />

        {#if form.errors.password}
          <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'red.500' })}>{form.errors.password}</div>
        {/if}
      </div>

      <div class={flex({ direction: 'column', gap: '4px' })}>
        <label class={css({ fontSize: '13px', color: 'gray.700', userSelect: 'none' })} for="confirmPassword">비밀번호 확인</label>
        <TextInput
          id="confirmPassword"
          aria-invalid={!!form.errors.confirmPassword}
          placeholder="비밀번호를 다시 입력하세요"
          type="password"
          bind:value={form.fields.confirmPassword}
        />

        {#if form.errors.confirmPassword}
          <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'red.500' })}>{form.errors.confirmPassword}</div>
        {/if}
      </div>
    </div>

    <Button style={css.raw({ height: '40px' })} loading={form.state.isLoading} size="lg" type="submit">비밀번호 변경하기</Button>
  </form>
</div>
