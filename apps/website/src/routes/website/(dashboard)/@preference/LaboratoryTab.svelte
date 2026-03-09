<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { SettingsCard } from '$lib/components';
  import { graphql } from '$mearie';
  import type { DashboardLayout_PreferenceModal_LaboratoryTab_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_PreferenceModal_LaboratoryTab_user$key;
  };

  let { user$key }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_LaboratoryTab_user on User {
        id
      }
    `),
    () => user$key,
  );
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <!-- Tab Header -->
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default', marginBottom: '12px' })}>실험실</h1>

    <div class={css({ fontSize: '14px', color: 'text.muted' })}>
      <p>실험실 기능은 아직 개발 중이거나 테스트 중인 기능들이에요.</p>
      <p class={css({ marginTop: '4px' })}>이 기능들은 언제든지 변경되거나 제거될 수 있어요.</p>
    </div>
  </div>

  <SettingsCard>
    <div class={css({ padding: '16px', fontSize: '14px', color: 'text.muted' })}>현재 활성화된 실험실 기능이 없어요.</div>
  </SettingsCard>
</div>
