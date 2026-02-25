<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { comma } from '@typie/ui/utils';
  import { SettingsCard, SettingsRow } from '$lib/components';
  import { graphql } from '$mearie';
  import type { DashboardLayout_PreferenceModal_RevenueTab_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_PreferenceModal_RevenueTab_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_RevenueTab_user on User {
        id
        revenue
      }
    `),
    () => user$key,
  );
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>수익</h1>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>누적 수익</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          현재 수익
        {/snippet}
        {#snippet description()}
          유료 콘텐츠 판매로 얻은 수익이에요.
        {/snippet}
        {#snippet value()}
          <span class={css({ fontSize: '16px', fontWeight: 'semibold' })}>{comma(user.data.revenue)}원</span>
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>출금</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          출금 계좌
        {/snippet}
        {#snippet description()}
          등록된 계좌가 없어요.
        {/snippet}
        {#snippet value()}
          <span class={css({ fontSize: '13px', color: 'text.muted' })}>준비 중</span>
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>
</div>
