<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Slider, Switch } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import type { DashboardLayout_PreferenceModal_EditorTab_user$key } from '$mearie';

  type Props = {
    user$key: DashboardLayout_PreferenceModal_EditorTab_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_EditorTab_user on User {
        id
        preferences
      }
    `),
    () => user$key,
  );

  const [updatePreferences] = createMutation(
    graphql(`
      mutation DashboardLayout_PreferenceModal_EditorTab_UpdatePreferences_Mutation($input: UpdatePreferencesInput!) {
        updatePreferences(input: $input) {
          id
          preferences
        }
      }
    `),
  );

  const app = getAppContext();

  const autoExcludeBulkEdits = $derived(user.data.preferences.autoExcludeBulkEdits !== false);
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <!-- Tab Header -->
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>에디터</h1>
  </div>

  <!-- Writing Position Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '4px' })}>시선 고정</h2>
    <p class={css({ fontSize: '13px', color: 'text.subtle', lineHeight: '[1.6]', marginBottom: '20px' })}>
      작성 중인 줄을 화면의 일정한 위치에 고정하여 목과 눈의 피로를 줄이고 집중력을 높일 수 있어요.
    </p>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          타자기 모드
        {/snippet}
        {#snippet description()}
          활성화하면 스크롤 시에도 작성 중인 줄이 화면에서 움직이지 않아요.
        {/snippet}
        {#snippet value()}
          <Switch
            onchange={() => {
              mixpanel.track('toggle_typewriter', {
                enabled: app.preference.current.typewriterEnabled,
              });
            }}
            bind:checked={app.preference.current.typewriterEnabled}
          />
        {/snippet}
      </SettingsRow>

      {#if app.preference.current.typewriterEnabled}
        <SettingsDivider />
        <SettingsRow vertical>
          {#snippet label()}
            고정 위치
          {/snippet}
          {#snippet description()}
            화면 상단부터 하단까지 원하는 높이를 선택할 수 있어요.
          {/snippet}
          {#snippet value()}
            <div class={flex({ width: 'full', align: 'center', gap: '16px' })}>
              <div class={css({ flexShrink: '0', fontSize: '12px', color: 'text.subtle', fontWeight: 'medium' })}>화면 상단</div>
              <Slider
                max={1}
                min={0}
                onchange={() => {
                  mixpanel.track('change_typewriter_position', {
                    position: Math.round(app.preference.current.typewriterPosition * 100),
                  });
                }}
                step={0.05}
                tooltipFormatter={(v) => `${Math.round(v * 100)}%`}
                bind:value={app.preference.current.typewriterPosition}
              />
              <div class={css({ flexShrink: '0', fontSize: '12px', color: 'text.subtle', fontWeight: 'medium' })}>화면 하단</div>
            </div>
          {/snippet}
        </SettingsRow>
      {/if}
    </SettingsCard>
  </div>

  <!-- Display Settings Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>시각 효과</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          현재 줄 강조
        {/snippet}
        {#snippet description()}
          작성 중인 줄에 배경색을 입혀 더 눈에 잘 띄게 해요.
        {/snippet}
        {#snippet value()}
          <Switch
            onchange={() => {
              mixpanel.track('toggle_line_highlight', {
                enabled: app.preference.current.lineHighlightEnabled,
              });
            }}
            bind:checked={app.preference.current.lineHighlightEnabled}
          />
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>

  <!-- Editing Settings Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>입력 보조</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          선택 영역 둘러싸기
        {/snippet}
        {#snippet description()}
          선택 영역 지정 후 따옴표나 괄호를 입력하면 둘러싸요.
        {/snippet}
        {#snippet value()}
          <Switch
            onchange={() => {
              mixpanel.track('toggle_auto_surround', {
                enabled: app.preference.current.autoSurroundEnabled,
              });
            }}
            bind:checked={app.preference.current.autoSurroundEnabled}
          />
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>

  <!-- Statistics Settings Section -->
  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>통계</h2>

    <SettingsCard>
      <SettingsRow>
        {#snippet label()}
          대량 편집 통계 자동 제외
        {/snippet}
        {#snippet description()}
          붙여넣기처럼 한 번에 큰 변경이 생기면 글자 수 통계에서 자동으로 제외해요. 타임라인에서 항목별로 바꿀 수 있어요.
        {/snippet}
        {#snippet value()}
          <Switch
            checked={autoExcludeBulkEdits}
            onclick={(e) => {
              e.preventDefault();

              if (!SubscribeModal.gate('preferences_editor')) {
                return;
              }

              const enabled = !autoExcludeBulkEdits;
              mixpanel.track('toggle_auto_exclude_bulk_edits', { enabled });
              void updatePreferences({ input: { value: { autoExcludeBulkEdits: enabled } } });
            }}
          />
        {/snippet}
      </SettingsRow>
    </SettingsCard>
  </div>
</div>
