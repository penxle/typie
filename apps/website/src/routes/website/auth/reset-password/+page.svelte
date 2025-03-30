<script lang="ts">
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { createForm, FormError } from '$lib/form';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

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
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'invalid_code') {
        throw new FormError('confirmPassword', '만료되었거나 유효하지 않은 링크입니다. 비밀번호 재설정을 다시 요청해주세요.');
      }
    },
  });
</script>

<div class={center({ width: 'screen', height: 'screen' })}>
  <div class={flex({ direction: 'column', gap: { base: '24px' }, maxWidth: '400px', width: 'full', padding: { base: '16px' } })}>
    <h1 class={css({ fontSize: { base: '24px' }, fontWeight: 'bold', textAlign: 'center' })}>비밀번호 재설정</h1>

    <p class={css({ textAlign: 'center' })}>새로운 비밀번호를 입력해주세요.</p>

    <form class={flex({ direction: 'column', gap: { base: '16px' } })} onsubmit={form.handleSubmit}>
      <div class={flex({ direction: 'column', gap: { base: '8px' } })}>
        <label for="password">새 비밀번호</label>
        <input
          id="password"
          class={css({ borderWidth: '1px', padding: '8px', borderRadius: '4px' })}
          placeholder="새 비밀번호를 입력하세요"
          type="password"
          bind:value={form.fields.password}
        />

        {#if form.errors.password}
          <p class={css({ color: 'red.500' })}>{form.errors.password}</p>
        {/if}
      </div>

      <div class={flex({ direction: 'column', gap: { base: '8px' } })}>
        <label for="confirmPassword">비밀번호 확인</label>
        <input
          id="confirmPassword"
          class={css({ borderWidth: '1px', padding: '8px', borderRadius: '4px' })}
          placeholder="비밀번호를 다시 입력하세요"
          type="password"
          bind:value={form.fields.confirmPassword}
        />

        {#if form.errors.confirmPassword}
          <p class={css({ color: 'red.500' })}>{form.errors.confirmPassword}</p>
        {/if}
      </div>

      <button
        class={css({ backgroundColor: '[#000000]', color: '[#FFFFFF]', padding: '12px', borderRadius: '4px', fontWeight: '[500]' })}
        disabled={form.state.isLoading}
        type="submit"
      >
        {form.state.isLoading ? '처리 중...' : '비밀번호 재설정'}
      </button>
    </form>
  </div>
</div>
