<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Switch } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { SettingsCard, SettingsRow } from '$lib/components';
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

  const app = getAppContext();
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
    <SettingsRow>
      {#snippet label()}
        v2 에디터 사용
      {/snippet}
      {#snippet description()}
        타이피 팀에서 새롭게 준비중인 에디터를 미리 체험해 볼 수 있어요.
        <br />
        아직 모든 기능 개발이 완료되지 않아, 실사용에는 적합하지 않아요.
        <br />
        새 문서를 만들 때 v2 에디터를 선택해 체험해보세요.
      {/snippet}
      {#snippet value()}
        <Switch
          onchange={() => {
            mixpanel.track('toggle_experimental_feature', {
              feature: 'v2_editor',
              enabled: app.preference.current.experimental_v2EditorEnabled,
            });
          }}
          bind:checked={app.preference.current.experimental_v2EditorEnabled}
        />
      {/snippet}
    </SettingsRow>
  </SettingsCard>
</div>
