<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Select } from '@typie/ui/components';
  import mixpanel from 'mixpanel-browser';
  import { fragment, graphql } from '$graphql';
  import type { DashboardLayout_PreferenceModal_ScreenTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_ScreenTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_ScreenTab_user on User {
        id
        preferences
      }
    `),
  );

  const updatePreferences = graphql(`
    mutation DashboardLayout_PreferenceModal_ScreenTab_UpdatePreferences_Mutation($input: UpdatePreferencesInput!) {
      updatePreferences(input: $input) {
        id
        preferences
      }
    }
  `);
</script>

<div class={flex({ direction: 'column', gap: '32px' })}>
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>화면</h1>

  <div class={flex({ align: 'center', justify: 'space-between', width: 'full', paddingY: '4px' })}>
    <div>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>첫 화면 설정</h3>
      <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>앱을 시작할 때 표시할 첫 화면을 설정합니다.</p>
    </div>

    <Select
      items={[
        { value: 'home', label: '홈 화면', description: '늘 홈 화면을 처음으로 표시합니다.' },
        { value: 'last', label: '마지막으로 본 항목', description: '마지막으로 본 항목을 자동으로 표시합니다.' },
      ]}
      onselect={async (value) => {
        mixpanel.track('change_initial_page', {
          page: value,
        });

        await updatePreferences({ value: { initialPage: value } });
      }}
      value={$user.preferences.initialPage ?? 'last'}
    />
  </div>
</div>
