<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { graphql } from '$graphql';
  import { Button, Modal, TextInput } from '$lib/components';
  import { createForm, FormError } from '$lib/form';
  import { Dialog } from '$lib/notification';

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
      email: z.string({ error: '이메일을 입력해주세요.' }).email('올바른 이메일 형식을 입력해주세요.'),
    }),
    onSubmit: async (data) => {
      await sendEmailUpdateEmail({ email: data.email });

      mixpanel.track('send_email_update_email');
      open = false;
      Dialog.alert({
        title: '이메일 변경',
        message: '변경할 이메일로 인증 메일을 발송했어요. 메일함을 확인해주세요.',
      });
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'user_email_exists') {
        throw new FormError('email', '이미 사용중인 이메일이에요.');
      }
    },
  });

  $effect(() => {
    void form;
  });
</script>

<Modal style={css.raw({ padding: '16px', maxWidth: '440px' })} bind:open>
  <p class={css({ paddingBottom: '20px', fontSize: '18px', textAlign: 'center', fontWeight: 'semibold' })}>이메일 변경</p>

  <div class={flex({ direction: 'column', gap: '36px' })}>
    <div>
      <p class={css({ marginBottom: '4px', fontSize: '15px', fontWeight: 'medium' })}>현재 이메일</p>
      <p class={css({ fontSize: '14px', color: 'text.muted' })}>{email}</p>
    </div>

    <form onsubmit={form.handleSubmit}>
      <label class={css({ display: 'block', marginBottom: '4px', fontSize: '15px', fontWeight: 'medium' })} for="email">
        변경할 이메일
      </label>
      <TextInput id="email" autofocus placeholder="new@example.com" type="email" bind:value={form.fields.email} />

      {#if form.errors.email}
        <div class={css({ marginTop: '4px', paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.email}</div>
      {/if}

      <Button style={css.raw({ marginTop: '12px', width: 'full' })} type="submit">변경</Button>
    </form>
  </div>
</Modal>
