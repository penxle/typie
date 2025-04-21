<script lang="ts">
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { page } from '$app/state';
  import Logo from '$assets/logos/logo.svg?component';
  import { graphql } from '$graphql';
  import { Button, TextInput } from '$lib/components';
  import { createForm, FormError } from '$lib/form';
  import { Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

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

<div class={flex({ flexDirection: 'column', gap: '24px' })}>
  <div class={flex({ justifyContent: 'flex-start' })}>
    <Logo class={css({ height: '20px' })} />
  </div>

  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <h1 class={css({ fontSize: '24px', fontWeight: 'extrabold' })}>비밀번호 재설정하기</h1>

    <div class={css({ fontSize: '14px', color: 'gray.500' })}>가입한 이메일을 입력하시면 비밀번호 재설정 링크를 보내드려요.</div>
  </div>

  <form class={flex({ flexDirection: 'column', gap: '24px' })} onsubmit={form.handleSubmit}>
    <div class={flex({ direction: 'column', gap: '12px' })}>
      <div class={flex({ direction: 'column', gap: '4px' })}>
        <label class={css({ fontSize: '13px', color: 'gray.700', userSelect: 'none' })} for="email">이메일</label>

        <TextInput id="email" aria-invalid={!!form.errors.email} placeholder="me@example.com" bind:value={form.fields.email} />

        {#if form.errors.email}
          <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'red.500' })}>{form.errors.email}</div>
        {/if}
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <Button style={css.raw({ height: '40px' })} loading={form.state.isLoading} size="lg" type="submit">비밀번호 재설정 링크 받기</Button>

      <div class={flex({ justifyContent: 'center' })}>
        <a
          class={css({ fontSize: '13px', color: 'gray.700', _hover: { textDecoration: 'underline', textUnderlineOffset: '2px' } })}
          href={`/login${page.url.search}`}
        >
          로그인 페이지로 돌아가기
        </a>
      </div>
    </div>
  </form>
</div>
