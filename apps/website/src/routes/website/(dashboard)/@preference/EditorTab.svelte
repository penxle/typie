<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Select, Slider, Switch } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { fragment, graphql } from '$graphql';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import type { DashboardLayout_PreferenceModal_EditorTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_EditorTab_user;
  };

  let { $user: _user }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_EditorTab_user on User {
        id
      }
    `),
  );

  const app = getAppContext();
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
          붙여넣기 옵션
        {/snippet}
        {#snippet description()}
          복사한 텍스트의 서식을 유지할지, 현재 문서 스타일로 통일할지 선택해요.
        {/snippet}
        {#snippet value()}
          <Select
            items={[
              { value: 'ask', label: '매번 묻기', description: '붙여넣기 시 선택해요.' },
              { value: 'html', label: '원본 서식 유지', description: '복사한 텍스트의 서식을 그대로 유지해요.' },
              { value: 'text', label: '문서 서식 적용', description: '현재 문서의 서식을 적용하여 붙여넣어요.' },
            ] as const}
            onselect={(value) => {
              mixpanel.track('change_paste_mode', {
                mode: app.preference.current.pasteMode,
              });

              app.preference.current.pasteMode = value;
            }}
            value={app.preference.current.pasteMode}
          />
        {/snippet}
      </SettingsRow>

      <SettingsDivider />

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
</div>
