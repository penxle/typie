<script lang="ts">
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { createForm, FormError } from '$lib/form';
  import { Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  const sendPasswordResetEmail = graphql(`
    mutation ForgotPasswordPage_SendPasswordResetEmail_Mutation($input: SendPasswordResetEmailInput!) {
      sendPasswordResetEmail(input: $input)
    }
  `);

  const form = createForm({
    schema: z.object({
      email: z.string({ required_error: '이메일을 입력해주세요.' }).email('올바른 이메일 형식을 입력해주세요.'),
    }),
    onSubmit: async (data) => {
      await sendPasswordResetEmail({
        email: data.email,
      });

      Toast.success('이메일을 보냈어요');
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'user_email_not_found') {
        throw new FormError('email', '등록되지 않은 이메일입니다.');
      }
    },
  });
</script>

<div class={center({ width: 'screen', height: 'screen' })}>
  <div class={flex({ direction: 'column', gap: { base: '24px' }, maxWidth: '400px', width: 'full', padding: { base: '16px' } })}>
    <h1 class={css({ fontSize: { base: '24px' }, fontWeight: 'bold', textAlign: 'center' })}>비밀번호 재설정</h1>

    <p class={css({ textAlign: 'center' })}>가입한 이메일 주소를 입력하시면 비밀번호 재설정 링크를 보내드립니다.</p>

    <form class={flex({ direction: 'column', gap: { base: '16px' } })} onsubmit={form.handleSubmit}>
      <div class={flex({ direction: 'column', gap: { base: '8px' } })}>
        <label for="email">이메일</label>
        <input
          id="email"
          class={css({ borderWidth: '1px', padding: '8px', borderRadius: '4px' })}
          placeholder="이메일을 입력하세요"
          type="text"
          bind:value={form.fields.email}
        />

        {#if form.errors.email}
          <p class={css({ color: 'red.500' })}>{form.errors.email}</p>
        {/if}
      </div>

      <button
        class={css({ backgroundColor: '[#000000]', color: '[#FFFFFF]', padding: '12px', borderRadius: '4px', fontWeight: '[500]' })}
        disabled={form.state.isLoading}
        type="submit"
      >
        {form.state.isLoading ? '처리 중...' : '비밀번호 재설정 링크 받기'}
      </button>
    </form>

    <div class={css({ textAlign: 'center', marginTop: '16px' })}>
      <p>
        <a class={css({ textDecoration: 'underline' })} href={`/login${page.url.search}`}>로그인 페이지로 돌아가기</a>
      </p>
    </div>
  </div>
</div>
