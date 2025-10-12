<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { redeemCodeSchema } from '@/validation';
  import { graphql } from '$graphql';

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

  $effect(() => {
    void form;
  });
</script>

<Modal style={css.raw({ padding: '24px', maxWidth: '440px' })} bind:open>
  <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>할인 코드 등록</h2>

  <form class={flex({ direction: 'column', gap: '20px' })} onsubmit={form.handleSubmit}>
    <div class={flex({ direction: 'column', gap: '8px' })}>
      <label class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })} for="code">할인 코드</label>
      <TextInput id="code" style={css.raw({ width: 'full' })} placeholder="할인 코드를 입력하세요" bind:value={form.fields.code} />

      {#if form.errors.code}
        <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.code}</div>
      {/if}
    </div>

    <div class={flex({ gap: '8px' })}>
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
      <Button style={css.raw({ flex: '1' })} type="submit">등록</Button>
    </div>
  </form>
</Modal>
