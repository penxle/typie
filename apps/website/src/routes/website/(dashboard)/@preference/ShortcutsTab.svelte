<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { fragment, graphql } from '$graphql';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import type { DashboardLayout_PreferenceModal_ShortcutsTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_ShortcutsTab_user;
  };

  let { $user: _user }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_ShortcutsTab_user on User {
        id
      }
    `),
  );

  type ShortcutCategory = {
    title: string;
    shortcuts: {
      keys: string[] | string[][];
      description: string;
    }[];
  };

  const isMac = typeof window !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.userAgent);
  const modKey = isMac ? 'Cmd' : 'Ctrl';
  const altKey = isMac ? 'Option' : 'Alt';

  const shortcutCategories: ShortcutCategory[] = [
    {
      title: '텍스트 서식',
      shortcuts: [
        { keys: [modKey, 'B'], description: '굵게' },
        { keys: [modKey, 'I'], description: '기울임' },
        { keys: [modKey, 'Shift', 'S'], description: '취소선' },
        { keys: [modKey, 'U'], description: '밑줄' },
      ],
    },
    {
      title: '편집',
      shortcuts: [
        { keys: [modKey, 'Z'], description: '실행 취소' },
        { keys: [modKey, 'Shift', 'Z'], description: '다시 실행' },
        { keys: [modKey, 'X'], description: '잘라내기' },
        { keys: [modKey, 'C'], description: '복사' },
        { keys: [modKey, 'V'], description: '붙여넣기' },
        { keys: [modKey, 'A'], description: '문단 선택 (반복시 전체 선택)' },
        { keys: [modKey, 'F'], description: '찾기, 바꾸기 열기' },
        { keys: [altKey, '↑'], description: '이전 문장 경계로 이동' },
        { keys: [altKey, '↓'], description: '다음 문장 경계로 이동' },
        { keys: ['Shift', altKey, '↑'], description: '이전 문장 경계까지 선택' },
        { keys: ['Shift', altKey, '↓'], description: '다음 문장 경계까지 선택' },
      ],
    },
    {
      title: '삽입',
      shortcuts: [
        { keys: ['Enter'], description: '문단 나누기' },
        { keys: ['Shift', 'Enter'], description: '줄바꿈' },
        { keys: [modKey, 'Enter'], description: '페이지 나누기' },
        { keys: [['드래그 앤 드롭'], [modKey, 'V']], description: '이미지/파일 삽입' },
        { keys: ['--'], description: '긴 대시 (—)' },
        { keys: ['...'], description: '말줄임표 (…)' },
        { keys: ['"'], description: '큰따옴표 (“”)' },
        { keys: ["'"], description: '작은따옴표 (‘’)' },
      ],
    },
    {
      title: '메뉴',
      shortcuts: [
        { keys: [modKey, 'K'], description: '빠른 검색 열기' },
        { keys: [modKey, 'J'], description: '노트 열기' },
        { keys: ['/'], description: '슬래시 메뉴 열기' },
        { keys: ['Esc'], description: '열린 메뉴 닫기' },
      ],
    },
    {
      title: '레이아웃',
      shortcuts: [
        { keys: [modKey, 'Shift', 'P'], description: '우측 패널 토글' },
        { keys: [modKey, 'Shift', 'M'], description: '집중 모드 전환' },
      ],
    },
  ];
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <!-- Tab Header -->
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>단축키</h1>
  </div>

  {#each shortcutCategories as category (category.title)}
    <!-- {category.title} Section -->
    <div>
      <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>{category.title}</h2>

      <SettingsCard>
        {#each category.shortcuts as shortcut, index (shortcut.description)}
          {#if index > 0}
            <SettingsDivider />
          {/if}
          <SettingsRow>
            {#snippet label()}
              {shortcut.description}
            {/snippet}
            {#snippet value()}
              <div
                class={flex({
                  align: 'center',
                  gap: '4px',
                  flexShrink: 0,
                })}
              >
                {#if Array.isArray(shortcut.keys[0])}
                  {#each shortcut.keys as keyGroup, groupIndex (groupIndex)}
                    {#if groupIndex > 0}
                      <span
                        class={css({
                          fontSize: '11px',
                          color: 'text.subtle',
                          fontWeight: 'medium',
                          marginX: '6px',
                        })}
                      >
                        또는
                      </span>
                    {/if}
                    <div class={flex({ align: 'center', gap: '4px' })}>
                      {#each keyGroup as key, keyIndex (key)}
                        {#if keyIndex > 0}
                          <span
                            class={css({
                              fontSize: '11px',
                              color: 'text.disabled',
                              fontWeight: 'normal',
                            })}
                          >
                            +
                          </span>
                        {/if}
                        <kbd
                          class={css({
                            display: 'inline-flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            minWidth: '24px',
                            height: '24px',
                            paddingX: '6px',
                            fontSize: '11px',
                            fontWeight: 'normal',
                            fontFamily: 'mono',
                            color: 'text.subtle',
                            borderWidth: '1px',
                            borderColor: 'border.subtle',
                            borderRadius: '4px',
                          })}
                        >
                          {key}
                        </kbd>
                      {/each}
                    </div>
                  {/each}
                {:else}
                  {#each shortcut.keys as key, keyIndex (key)}
                    {#if keyIndex > 0}
                      <span
                        class={css({
                          fontSize: '11px',
                          color: 'text.disabled',
                          fontWeight: 'medium',
                        })}
                      >
                        +
                      </span>
                    {/if}
                    <kbd
                      class={css({
                        display: 'inline-flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        minWidth: '24px',
                        height: '24px',
                        paddingX: '6px',
                        fontSize: '11px',
                        fontWeight: 'normal',
                        fontFamily: 'mono',
                        color: 'text.subtle',
                        borderWidth: '1px',
                        borderColor: 'border.subtle',
                        borderRadius: '4px',
                      })}
                    >
                      {key}
                    </kbd>
                  {/each}
                {/if}
              </div>
            {/snippet}
          </SettingsRow>
        {/each}
      </SettingsCard>
    </div>
  {/each}
</div>
