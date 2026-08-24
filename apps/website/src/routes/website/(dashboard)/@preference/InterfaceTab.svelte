<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Select } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import type { DashboardLayout_PreferenceModal_InterfaceTab_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_PreferenceModal_InterfaceTab_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_InterfaceTab_user on User {
        id
        preferences
      }
    `),
    () => user$key,
  );

  const [updatePreferences] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_InterfaceTab_UpdatePreferences_Mutation($input: UpdatePreferencesInput!) {
        updatePreferences(input: $input) {
          id
          preferences
        }
      }
    `),
  );

  const app = getAppContext();
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <!-- Tab Header -->
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>인터페이스</h1>
  </div>

  <!-- Screen Settings Section -->
  <div>
    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          첫 화면
        {/snippet}
        {#snippet description()}
          타이피를 열었을 때 가장 먼저 보이는 화면을 선택해요.
        {/snippet}
        {#snippet value()}
          <Select
            items={[
              { value: 'home', label: '홈 화면', description: '늘 홈 화면을 처음으로 표시해요.' },
              { value: 'last', label: '마지막으로 본 항목', description: '이전에 보던 페이지를 자동으로 열어요.' },
            ]}
            onselect={async (value) => {
              if (!SubscribeModal.gate('preferences_interface')) {
                return;
              }

              mixpanel.track('change_initial_page', {
                page: value,
              });

              await updatePreferences({ input: { value: { initialPage: value } } });
            }}
            value={user.data.preferences.initialPage ?? 'last'}
          />
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          새 문서의 기본 툴바
        {/snippet}
        {#snippet description()}
          툴바를 아직 맞바꾸지 않은 문서에서 항상 보이는 툴바예요.
        {/snippet}
        {#snippet value()}
          <Select
            items={[
              { value: 'format', label: '서식', description: '글꼴·굵기·색 같은 서식 도구가 항상 보여요.' },
              { value: 'insert', label: '삽입', description: '이미지·표·목록 같은 삽입 도구가 항상 보여요.' },
            ]}
            onselect={async (value) => {
              if (!SubscribeModal.gate('preferences_interface')) {
                return;
              }

              mixpanel.track('change_default_primary_toolbar', {
                kind: value,
              });

              app.preference.current.defaultPrimaryToolbar = value;
              await updatePreferences({ input: { value: { defaultPrimaryToolbar: value } } });
            }}
            value={user.data.preferences.defaultPrimaryToolbar ?? 'format'}
          />
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>
</div>
