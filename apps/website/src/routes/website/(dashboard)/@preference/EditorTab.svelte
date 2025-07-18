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

  const appContext = getAppContext();
  const { preference } = appContext;

  let typewriterEnabled = $state(preference.current.typewriterEnabled ?? false);
  let typewriterPosition = $state(preference.current.typewriterPosition ?? 0.5);
  let lineHighlightEnabled = $state(preference.current.lineHighlightEnabled ?? true);

  $effect(() => {
    preference.current.typewriterEnabled = typewriterEnabled;
    preference.current.typewriterPosition = typewriterPosition;
    preference.current.lineHighlightEnabled = lineHighlightEnabled;
  });

  const handleTypewriterToggle = () => {
    mixpanel.track('toggle_typewriter', {
      enabled: typewriterEnabled,
    });
  };

  const handleTypewriterPositionChange = () => {
    if (typewriterEnabled) {
      mixpanel.track('change_typewriter_position', {
        position: Math.round(typewriterPosition * 100),
      });
    }
  };

  const handleLineHighlightToggle = () => {
    mixpanel.track('toggle_line_highlight', {
      enabled: lineHighlightEnabled,
    });
  };
</script>

<div class={flex({ direction: 'column', gap: '32px' })}>
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>에디터</h1>

  <div class={flex({ align: 'center', justify: 'space-between', width: 'full', paddingY: '4px' })}>
    <div>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>타자기 모드</h3>
      <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>
        타자기 모드를 활성화하면 현재 작성 중인 줄이 항상 화면의 특정 위치에 고정됩니다.
      </p>
    </div>

    <Switch onchange={handleTypewriterToggle} bind:checked={typewriterEnabled} />
  </div>

  {#if typewriterEnabled}
    <div class={flex({ direction: 'column', gap: '16px' })}>
      <div class={flex({ direction: 'column', gap: '4px' })}>
        <div class={css({ fontSize: '14px', fontWeight: 'medium' })}>고정 위치</div>
        <div class={css({ fontSize: '13px', color: 'text.muted' })}>현재 작성 중인 줄이 고정될 화면상의 위치를 설정합니다.</div>
      </div>

      <div class={flex({ width: 'full', align: 'center', gap: '16px' })}>
        <div class={css({ flexShrink: '0', fontSize: '13px', color: 'text.muted', fontWeight: 'medium' })}>화면 상단</div>
        <Slider
          max={1}
          min={0}
          onchange={handleTypewriterPositionChange}
          step={0.05}
          tooltipFormatter={(v) => `${Math.round(v * 100)}%`}
          bind:value={typewriterPosition}
        />
        <div class={css({ flexShrink: '0', fontSize: '13px', color: 'text.muted', fontWeight: 'medium' })}>화면 하단</div>
      </div>
    </div>
  {/if}

  <div class={flex({ align: 'center', justify: 'space-between', width: 'full', paddingY: '4px' })}>
    <div>
      <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>현재 줄 강조</h3>
      <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>현재 작성 중인 줄을 강조하여 화면에 표시합니다.</p>
    </div>

    <Switch onchange={handleLineHighlightToggle} bind:checked={lineHighlightEnabled} />
  </div>
</div>
