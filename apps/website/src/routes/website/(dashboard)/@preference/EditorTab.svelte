<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { fragment, graphql } from '$graphql';
  import { Slider, Switch } from '$lib/components';
  import { getAppContext } from '$lib/context/app.svelte';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
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

<div class={flex({ direction: 'column', gap: '32px' })}>
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>에디터</h1>

  <div class={flex({ align: 'center', justify: 'space-between', width: 'full', paddingY: '4px' })}>
    <div>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>타자기 모드</h3>
      <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>
        현재 작성 중인 줄을 항상 화면의 특정 위치에 고정합니다.
      </p>
    </div>

    <Switch
      onchange={() => {
        mixpanel.track('toggle_typewriter', {
          enabled: app.preference.current.typewriterEnabled,
        });
      }}
      bind:checked={app.preference.current.typewriterEnabled}
    />
  </div>

  {#if app.preference.current.typewriterEnabled}
    <div class={flex({ direction: 'column', gap: '16px' })}>
      <div class={flex({ direction: 'column', gap: '4px' })}>
        <div class={css({ fontSize: '14px', fontWeight: 'medium' })}>고정 위치</div>
        <div class={css({ fontSize: '13px', color: 'text.faint' })}>현재 작성 중인 줄이 고정될 화면상의 위치를 설정합니다.</div>
      </div>

      <div class={flex({ width: 'full', align: 'center', gap: '16px' })}>
        <div class={css({ flexShrink: '0', fontSize: '13px', color: 'text.faint', fontWeight: 'medium' })}>화면 상단</div>
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
        <div class={css({ flexShrink: '0', fontSize: '13px', color: 'text.faint', fontWeight: 'medium' })}>화면 하단</div>
      </div>
    </div>
  {/if}

  <div class={flex({ align: 'center', justify: 'space-between', width: 'full', paddingY: '4px' })}>
    <div>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>현재 줄 강조</h3>
      <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>현재 작성 중인 줄을 강조하여 화면에 표시합니다.</p>
    </div>

    <Switch
      onchange={() => {
        mixpanel.track('toggle_line_highlight', {
          enabled: app.preference.current.lineHighlightEnabled,
        });
      }}
      bind:checked={app.preference.current.lineHighlightEnabled}
    />
  </div>
</div>
