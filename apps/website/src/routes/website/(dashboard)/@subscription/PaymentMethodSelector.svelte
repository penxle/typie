<script lang="ts">
  import { BillingKeyType } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Tooltip } from '@typie/ui/components';
  import KakaoPayLogo from '$assets/icons/kakaopay.svg?component';

  type Props = {
    method: BillingKeyType;
    disabled?: Partial<Record<BillingKeyType, string>>;
  };

  let { method = $bindable(), disabled }: Props = $props();

  const options = [
    { value: BillingKeyType.CARD, label: '신용·체크카드' },
    { value: BillingKeyType.KAKAOPAY, label: '카카오페이' },
  ] as const;
</script>

<div class={flex({ direction: 'column', gap: '8px' })}>
  <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>결제 수단</div>
  <div class={flex({ gap: '8px' })}>
    {#each options as option (option.value)}
      {@const reason = disabled?.[option.value]}
      <Tooltip
        style={css.raw({ flex: '1', cursor: reason ? 'not-allowed' : undefined })}
        enabled={!!reason}
        message={reason}
        placement="top"
      >
        <button
          class={flex({
            width: 'full',
            alignItems: 'center',
            justify: 'center',
            gap: '6px',
            borderRadius: '6px',
            borderWidth: '1px',
            borderColor: method === option.value ? 'accent.brand.default' : 'border.subtle',
            padding: '12px',
            fontSize: '13px',
            fontWeight: 'medium',
            color: reason ? 'text.disabled' : 'text.default',
            backgroundColor: 'surface.default',
            cursor: reason ? 'not-allowed' : 'pointer',
            opacity: reason ? '50' : '100',
            transition: 'common',
            _hover: {
              borderColor: reason ? 'border.subtle' : method === option.value ? 'accent.brand.default' : 'border.default',
            },
          })}
          aria-label={option.value === BillingKeyType.KAKAOPAY ? option.label : undefined}
          aria-pressed={method === option.value}
          disabled={!!reason}
          onclick={() => (method = option.value)}
          type="button"
        >
          {#if option.value === BillingKeyType.KAKAOPAY}
            <KakaoPayLogo class={css({ height: '16px' })} />
          {:else}
            {option.label}
          {/if}
        </button>
      </Tooltip>
    {/each}
  </div>
</div>
