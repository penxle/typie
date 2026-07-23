<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Checkbox, TextInput } from '@typie/ui/components';

  type CardFields = {
    cardNumber?: string;
    expiryDate?: string;
    passwordTwoDigits?: string;
    birthOrBusinessRegistrationNumber?: string;
    agreementsAccepted: boolean;
  };

  type Props = {
    fields: CardFields;
    errors: Partial<Record<keyof CardFields, string>>;
    showCardFields?: boolean;
  };

  let { fields, errors, showCardFields = true }: Props = $props();

  const agreements = [
    { name: '타이피 결제 이용약관', url: 'https://typie.co/legal/terms' },
    { name: 'NICEPAY 전자금융거래 기본약관', url: 'https://www.nicepay.co.kr/cs/terms/policy1.do' },
  ];

  let agreementChecks = $state(agreements.map(() => false));
  const allChecked = $derived(agreementChecks.every(Boolean));

  $effect(() => {
    fields.agreementsAccepted = allChecked;
  });

  const handleAllCheck = () => {
    agreementChecks = agreementChecks.map(() => !allChecked);
  };

  const formatBusinessNumber = (event: Event) => {
    const input = event.target as HTMLInputElement;
    const value = input.value.replaceAll(/\D/g, '');

    if (value.length <= 6) {
      input.value = value;
    } else {
      const parts = [value.slice(0, 3), value.slice(3, 5), value.slice(5)];
      input.value = parts.filter(Boolean).join('-');
    }
  };

  const formatCardNumber = (event: Event) => {
    const input = event.target as HTMLInputElement;
    const value = input.value.replaceAll(/\D/g, '');
    const parts = [value.slice(0, 4), value.slice(4, 8), value.slice(8, 12), value.slice(12)];
    input.value = parts.filter(Boolean).join('-');
  };

  const formatCardExpiry = (event: Event) => {
    const input = event.target as HTMLInputElement;
    const value = input.value.replaceAll(/\D/g, '');
    input.value = value.length > 2 ? value.slice(0, 2) + '/' + value.slice(2, 4) : value;
  };
</script>

{#if showCardFields}
  <div class={flex({ direction: 'column', gap: '8px' })}>
    <TextInput
      id="cardNumber"
      style={css.raw({ width: 'full' })}
      inputmode="numeric"
      maxlength={19}
      oninput={formatCardNumber}
      placeholder="카드 번호"
      bind:value={fields.cardNumber}
    />
    {#if errors.cardNumber}
      <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{errors.cardNumber}</div>
    {/if}
  </div>

  <div class={flex({ gap: '8px' })}>
    <div class={flex({ direction: 'column', gap: '8px', flex: '1' })}>
      <TextInput
        id="expiryDate"
        style={css.raw({ width: 'full' })}
        inputmode="numeric"
        maxlength={5}
        oninput={formatCardExpiry}
        placeholder="유효기간 (MM/YY)"
        bind:value={fields.expiryDate}
      />
      {#if errors.expiryDate}
        <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{errors.expiryDate}</div>
      {/if}
    </div>

    <div class={flex({ direction: 'column', gap: '8px', flex: '1' })}>
      <TextInput
        id="passwordTwoDigits"
        style={css.raw({ width: 'full' })}
        autocomplete="off"
        inputmode="numeric"
        maxlength={2}
        placeholder="비밀번호 앞 2자리"
        type="password"
        bind:value={fields.passwordTwoDigits}
      />
      {#if errors.passwordTwoDigits}
        <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{errors.passwordTwoDigits}</div>
      {/if}
    </div>
  </div>

  <div class={flex({ direction: 'column', gap: '8px' })}>
    <TextInput
      id="birthOrBusinessRegistrationNumber"
      style={css.raw({ width: 'full' })}
      inputmode="numeric"
      maxlength={12}
      oninput={formatBusinessNumber}
      placeholder="생년월일 6자리 또는 사업자번호 10자리"
      bind:value={fields.birthOrBusinessRegistrationNumber}
    />
    {#if errors.birthOrBusinessRegistrationNumber}
      <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>
        {errors.birthOrBusinessRegistrationNumber}
      </div>
    {/if}
  </div>
{/if}

<div class={flex({ direction: 'column', gap: '8px' })}>
  <div
    class={css({
      borderRadius: '8px',
      borderWidth: '1px',
      borderColor: 'border.subtle',
      padding: '16px',
      backgroundColor: 'surface.default',
    })}
  >
    <div class={flex({ direction: 'column', gap: '12px' })}>
      <Checkbox checked={allChecked} onchange={handleAllCheck} size="sm">
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>전체 동의</span>
      </Checkbox>

      <div class={css({ height: '1px', backgroundColor: 'border.subtle' })}></div>

      <div class={flex({ direction: 'column', gap: '8px' })}>
        {#each agreements as agreement (agreement.name)}
          <Checkbox size="sm" bind:checked={agreementChecks[agreements.indexOf(agreement)]}>
            <span class={css({ fontSize: '13px', color: 'text.subtle' })}>
              <a
                class={css({ color: 'text.default', textDecoration: 'underline', _hover: { color: 'accent.brand.default' } })}
                href={agreement.url}
                rel="noopener noreferrer"
                target="_blank"
              >
                {agreement.name}
              </a>
              동의 (필수)
            </span>
          </Checkbox>
        {/each}
      </div>
    </div>
  </div>

  {#if errors.agreementsAccepted}
    <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{errors.agreementsAccepted}</div>
  {/if}
</div>
