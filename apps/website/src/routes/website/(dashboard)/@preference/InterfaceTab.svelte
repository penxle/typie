<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Select } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { fragment, graphql } from '$graphql';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import type { DashboardLayout_PreferenceModal_InterfaceTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_InterfaceTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_InterfaceTab_user on User {
        id
        preferences
      }
    `),
  );

  const updatePreferences = graphql(`
    mutation DashboardLayout_PreferenceModal_InterfaceTab_UpdatePreferences_Mutation($input: UpdatePreferencesInput!) {
      updatePreferences(input: $input) {
        id
        preferences
      }
    }
  `);

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
              mixpanel.track('change_initial_page', {
                page: value,
              });

              await updatePreferences({ value: { initialPage: value } });
            }}
            value={$user.preferences.initialPage ?? 'last'}
          />
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          툴바 스타일
        {/snippet}
        {#snippet description()}
          상단 툴바 스타일을 고를 수 있어요.
        {/snippet}
        {#snippet value()}
          <Select
            items={[
              { value: 'compact', label: '컴팩트', description: '아이콘만 표시하는 간결한 스타일이에요.' },
              { value: 'classic', label: '클래식', description: '아이콘과 텍스트를 함께 보여주는 스타일이에요.' },
            ]}
            onselect={async (value) => {
              mixpanel.track('change_toolbar_style', {
                style: value,
              });

              app.preference.current.toolbarStyle = value;
              await updatePreferences({ value: { toolbarStyle: value } });
            }}
            value={$user.preferences.toolbarStyle ?? 'compact'}
          />
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

      <SettingsRow>
        {#snippet label()}
          사이드바 자동 표시 방법
        {/snippet}
        {#snippet description()}
          숨김 모드일 때 마우스 호버 또는 클릭으로 표시할 수 있어요.
        {/snippet}
        {#snippet value()}
          <Select
            items={[
              { value: 'hover', label: '호버', description: '왼쪽 가장자리에 마우스를 올려 표시해요.' },
              { value: 'click', label: '클릭', description: '왼쪽 가장자리의 힌트를 클릭해 표시해요.' },
            ]}
            onselect={async (value) => {
              mixpanel.track('change_sidebar_trigger', {
                trigger: value,
              });

              app.preference.current.sidebarTrigger = value;
              await updatePreferences({ value: { sidebarTrigger: value } });
            }}
            value={$user.preferences.sidebarTrigger ?? 'hover'}
          />
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>
</div>
