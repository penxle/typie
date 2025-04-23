<script lang="ts">
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { graphql } from '$graphql';
  import { Button, Modal, TextInput } from '$lib/components';
  import { createForm, FormError } from '$lib/form';
  import { Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

  type Props = {
    open: boolean;
    email: string;
  };

  let { open = $bindable(), email }: Props = $props();

  const sendEmailUpdateEmail = graphql(`
    mutation DashboardLayout_UpdateEmailModal_SendEmailUpdateEmail_Mutation($input: SendEmailUpdateEmailInput!) {
      sendEmailUpdateEmail(input: $input)
    }
  `);

  const form = createForm({
    schema: z.object({
      email: z.string({ required_error: '이메일을 입력해주세요.' }).email('올바른 이메일 형식을 입력해주세요.'),
    }),
    onSubmit: async (data) => {
      await sendEmailUpdateEmail({ email: data.email });

      open = false;
      Toast.success('이메일 변경을 위한 확인 메일이 전송되었어요.');
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'user_email_exists') {
        throw new FormError('email', '이미 사용중인 이메일이에요.');
      }
    },
  });
</script>

<Modal style={css.raw({ maxWidth: '440px' })} bind:open>
  <p class={css({ paddingBottom: '20px', fontSize: '18px', textAlign: 'center', fontWeight: 'semibold' })}>이메일 변경</p>

  <div class={flex({ direction: 'column', gap: '36px' })}>
    <div>
      <p class={css({ marginBottom: '4px', fontSize: '15px', fontWeight: 'medium' })}>현재 이메일</p>
      <p class={css({ fontSize: '14px', color: 'gray.600' })}>{email}</p>
    </div>

    <form onsubmit={form.handleSubmit}>
      <label class={css({ display: 'block', marginBottom: '4px', fontSize: '15px', fontWeight: 'medium' })} for="email">
        변경할 이메일
      </label>
      <TextInput id="email" autofocus placeholder="new@example.com" type="email" bind:value={form.fields.email} />

      {#if form.errors.email}
        <div class={css({ marginTop: '4px', paddingLeft: '4px', fontSize: '12px', color: 'red.500' })}>{form.errors.email}</div>
      {/if}

      <Button style={css.raw({ marginTop: '12px', width: 'full' })} type="submit">변경</Button>
    </form>
  </div>
</Modal>
