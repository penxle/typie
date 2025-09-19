<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Select, Slider, Switch } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { fragment, graphql } from '$graphql';
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

<div class={flex({ direction: 'column', gap: '36px' })}>
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>에디터</h1>

  <div class={flex({ direction: 'column', gap: '20px' })}>
    <h2 class={css({ fontSize: '16px', fontWeight: 'bold', color: 'text.subtle' })}>작성 위치</h2>

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
  </div>

  <div class={flex({ direction: 'column', gap: '20px' })}>
    <h2 class={css({ fontSize: '16px', fontWeight: 'bold', color: 'text.subtle' })}>표시 설정</h2>

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

  <div class={flex({ direction: 'column', gap: '20px' })}>
    <h2 class={css({ fontSize: '16px', fontWeight: 'bold', color: 'text.subtle' })}>편집 설정</h2>

    <div class={flex({ align: 'center', justify: 'space-between', width: 'full', paddingY: '4px' })}>
      <div>
        <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>붙여넣기 옵션</h3>
        <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>
          붙여넣기 시 텍스트를 어떤 형식으로 붙여넣을지 설정합니다.
        </p>
      </div>

      <Select
        items={[
          { value: 'ask', label: '매번 묻기', description: '붙여넣기 시 선택합니다.' },
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
    </div>

    <div class={flex({ align: 'center', justify: 'space-between', width: 'full', paddingY: '4px' })}>
      <div>
        <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>선택 영역 둘러싸기</h3>
        <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>
          선택 영역 지정 후 따옴표나 괄호를 입력하면 둘러쌉니다.
        </p>
      </div>

      <Switch
        onchange={() => {
          mixpanel.track('toggle_auto_surround', {
            enabled: app.preference.current.autoSurroundEnabled,
          });
        }}
        bind:checked={app.preference.current.autoSurroundEnabled}
      />
    </div>
  </div>
</div>
