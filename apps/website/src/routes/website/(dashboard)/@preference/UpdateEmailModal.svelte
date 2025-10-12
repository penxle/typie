<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Dialog } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { graphql } from '$graphql';

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

<Modal style={css.raw({ padding: '24px', maxWidth: '480px' })} bind:open>
  <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>이메일 변경</h2>

  <form class={flex({ direction: 'column', gap: '24px' })} onsubmit={form.handleSubmit}>
    <div class={flex({ direction: 'column', gap: '8px' })}>
      <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>현재 이메일</div>
      <div
        class={css({
          padding: '12px',
          borderRadius: '6px',
          backgroundColor: 'surface.muted',
          fontSize: '14px',
          color: 'text.subtle',
        })}
      >
        {email}
      </div>
    </div>

    <div class={flex({ direction: 'column', gap: '8px' })}>
      <label class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })} for="email">새 이메일</label>
      <TextInput id="email" autofocus placeholder="new@example.com" type="email" bind:value={form.fields.email} />

      {#if form.errors.email}
        <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.email}</div>
      {/if}
    </div>

    <div class={flex({ gap: '8px', marginTop: '8px' })}>
      <Button
        style={css.raw({ flex: '1' })}
        onclick={() => {
          open = false;
        }}
        type="button"
        variant="secondary"
      >
        취소
      </Button>
      <Button style={css.raw({ flex: '1' })} type="submit">변경</Button>
    </div>
  </form>
</Modal>
