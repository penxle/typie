<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { TextInput } from '@typie/ui/components';

  type CardFields = {
    cardNumber?: string;
    expiryDate?: string;
    passwordTwoDigits?: string;
    birthOrBusinessRegistrationNumber?: string;
  };

  type Props = {
    fields: CardFields;
    errors: Partial<Record<keyof CardFields, string>>;
  };

  let { fields, errors }: Props = $props();

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
