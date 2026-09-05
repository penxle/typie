<script lang="ts">
  import { BillingKeyType } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Checkbox } from '@typie/ui/components';
  import { untrack } from 'svelte';

  type Props = {
    error?: string;
    method?: BillingKeyType;
    onchange: (accepted: boolean) => void;
  };

  let { error, method = BillingKeyType.CARD, onchange }: Props = $props();

  const agreements = $derived(
    method === BillingKeyType.KAKAOPAY
      ? [{ name: '타이피 결제 이용약관', url: 'https://typie.co/legal/terms' }]
      : [
          { name: '타이피 결제 이용약관', url: 'https://typie.co/legal/terms' },
          { name: 'NICEPAY 전자금융거래 기본약관', url: 'https://www.nicepay.co.kr/cs/terms/policy1.do' },
        ],
  );

  let checkedNames = $state<string[]>([]);
  const allChecked = $derived(agreements.every((agreement) => checkedNames.includes(agreement.name)));

  $effect(() => {
    void agreements;
    untrack(() => {
      checkedNames = [];
    });
  });

  $effect(() => {
    onchange(allChecked);
  });

  const toggle = (name: string) => {
    checkedNames = checkedNames.includes(name) ? checkedNames.filter((it) => it !== name) : [...checkedNames, name];
  };

  const handleAllCheck = () => {
    checkedNames = allChecked ? [] : agreements.map((agreement) => agreement.name);
  };
</script>

<div class={flex({ direction: 'column', gap: '8px' })}>
  <div
    class={css({
      borderRadius: '8px',
      borderWidth: '1px',
      borderColor: 'border.hairline',
      padding: '16px',
      backgroundColor: 'surface.default',
    })}
  >
    <div class={flex({ direction: 'column', gap: '12px' })}>
      <Checkbox checked={allChecked} onchange={handleAllCheck} size="sm">
        <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>전체 동의</span>
      </Checkbox>

      <div class={css({ height: '1px', backgroundColor: 'border.hairline' })}></div>

      <div class={flex({ direction: 'column', gap: '8px' })}>
        {#each agreements as agreement (agreement.name)}
          <Checkbox checked={checkedNames.includes(agreement.name)} onchange={() => toggle(agreement.name)} size="sm">
            <span class={css({ fontSize: '13px', color: 'text.muted' })}>
              <a
                class={css({ color: 'text.default', textDecoration: 'underline' })}
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

  {#if error}
    <div class={css({ paddingLeft: '4px', fontSize: '12px', color: 'danger.default' })}>{error}</div>
  {/if}
</div>
