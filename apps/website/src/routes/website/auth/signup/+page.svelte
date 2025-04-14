<script lang="ts">
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { graphql } from '$graphql';
  import { createForm, FormError } from '$lib/form';
  import { Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  const sendSignUpEmail = graphql(`
    mutation SignUpPage_SendSignUpEmail_Mutation($input: SendSignUpEmailInput!) {
      sendSignUpEmail(input: $input)
    }
  `);

  const form = createForm({
    schema: z
      .object({
        name: z.string({ required_error: '이름을 입력해주세요.' }).nonempty('이름을 입력해주세요.'),
        email: z.string({ required_error: '이메일을 입력해주세요.' }).email('올바른 이메일 형식을 입력해주세요.'),
        password: z.string({ required_error: '비밀번호를 입력해주세요.' }).nonempty('비밀번호를 입력해주세요.'),
        confirmPassword: z.string({ required_error: '비밀번호 확인을 입력해주세요.' }).nonempty('비밀번호 확인을 입력해주세요.'),
      })
      .refine((data) => data.password === data.confirmPassword, {
        path: ['confirmPassword'],
        message: '비밀번호가 일치하지 않습니다.',
      }),
    onSubmit: async (data) => {
      await sendSignUpEmail({
        email: data.email,
        name: data.name,
        password: data.password,
      });

      Toast.success('이메일을 보냈어요');
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'user_email_exists') {
        throw new FormError('email', '이미 사용중인 이메일입니다.');
      }
    },
  });
</script>

<div class={center({ width: 'screen', height: 'screen' })}>
  <div class={flex({ direction: 'column', gap: { base: '24px' }, maxWidth: '400px', width: 'full', padding: { base: '16px' } })}>
    <h1 class={css({ fontSize: { base: '24px' }, fontWeight: 'bold', textAlign: 'center' })}>회원가입</h1>

    <form class={flex({ direction: 'column', gap: { base: '16px' } })} onsubmit={form.handleSubmit}>
      <div class={flex({ direction: 'column', gap: { base: '8px' } })}>
        <label for="name">이름</label>
        <input
          id="name"
          class={css({ borderWidth: '1px', padding: '8px', borderRadius: '4px' })}
          placeholder="이름을 입력하세요"
          type="text"
          bind:value={form.fields.name}
        />

        {#if form.errors.name}
          <p class={css({ color: 'red.500' })}>{form.errors.name}</p>
        {/if}
      </div>

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

      <div class={flex({ direction: 'column', gap: { base: '8px' } })}>
        <label for="password">비밀번호</label>
        <input
          id="password"
          class={css({ borderWidth: '1px', padding: '8px', borderRadius: '4px' })}
          placeholder="비밀번호를 입력하세요"
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
        {form.state.isLoading ? '처리 중...' : '가입하기'}
      </button>
    </form>
  </div>
</div>
