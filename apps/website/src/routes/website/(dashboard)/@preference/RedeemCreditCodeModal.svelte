<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { redeemCodeSchema } from '@/validation';
  import { graphql } from '$graphql';
  import { Button, Modal, TextInput } from '$lib/components';
  import { createForm, FormError } from '$lib/form';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

  type Props = {
    open: boolean;
  };

  let { open = $bindable() }: Props = $props();

  const redeemCreditCode = graphql(`
    mutation DashboardLayout_PreferenceModal_BillingTab_RedeemCreditCodeModal_RedeemCreditCode_Mutation($input: RedeemCreditCodeInput!) {
      redeemCreditCode(input: $input) {
        id
        credit
      }
    }
  `);

  const form = createForm({
    schema: z.object({
      code: redeemCodeSchema,
    }),
    onSubmit: async (data) => {
      await redeemCreditCode({ code: data.code });
      mixpanel.track('redeem_credit_code', { via: 'redeem-credit-code-modal' });

      open = false;
    },
    onError: (error) => {
      if (error instanceof TypieError) {
        if (error.code === 'invalid_code') {
          throw new FormError('code', '유효하지 않은 할인 코드입니다.');
        } else if (error.code === 'already_redeemed') {
          throw new FormError('code', '이미 등록된 할인 코드입니다.');
        }
      }
    },
  });
</script>

<Modal style={css.raw({ gap: '24px', padding: '20px', maxWidth: '500px' })} bind:open>
  <p class={css({ fontWeight: 'semibold' })}>할인코드 등록</p>

  <form class={flex({ align: 'flex-start', gap: '4px' })} onsubmit={form.handleSubmit}>
    <div class={css({ width: 'full' })}>
      <TextInput id="code" style={css.raw({ width: 'full' })} placeholder="할인 코드 입력하기" size="sm" bind:value={form.fields.code} />

      {#if form.errors.code}
        <div class={css({ marginTop: '4px', paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.code}</div>
      {/if}
    </div>

    <Button style={css.raw({ flex: 'none' })} size="sm" type="submit" variant="secondary">등록</Button>
  </form>
</Modal>
