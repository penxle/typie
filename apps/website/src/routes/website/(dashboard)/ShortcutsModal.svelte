<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import ArrowBigUpIcon from '~icons/lucide/arrow-big-up';
  import CommandIcon from '~icons/lucide/command';
  import OptionIcon from '~icons/lucide/option';
  import type { Component } from 'svelte';

  const app = getAppContext();

  const isMac = typeof window !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.userAgent);

  type Key = string | { icon: Component };

  const mod: Key = isMac ? { icon: CommandIcon } : 'Ctrl';
  const alt: Key = isMac ? { icon: OptionIcon } : 'Alt';
  const shift: Key = isMac ? { icon: ArrowBigUpIcon } : 'Shift';

  type Category = { title: string; shortcuts: { keys: Key[]; label: string }[] };

  const left: Category[] = [
    {
      title: '텍스트 서식',
      shortcuts: [
        { keys: [mod, 'B'], label: '굵게' },
        { keys: [mod, 'I'], label: '기울임' },
        { keys: [mod, shift, 'S'], label: '취소선' },
        { keys: [mod, 'U'], label: '밑줄' },
      ],
    },
    {
      title: '삽입',
      shortcuts: [
        { keys: ['Enter'], label: '문단 나누기' },
        { keys: [shift, 'Enter'], label: '줄바꿈' },
        { keys: [mod, 'Enter'], label: '페이지 나누기' },
      ],
    },
  ];

  const right: Category[] = [
    {
      title: '편집',
      shortcuts: [
        { keys: [mod, 'Z'], label: '실행 취소' },
        { keys: [mod, shift, 'Z'], label: '다시 실행' },
        { keys: [mod, 'A'], label: '문단/전체 선택' },
        { keys: [mod, 'F'], label: '찾기/바꾸기' },
        { keys: [alt, '↑↓'], label: '문장 경계 이동' },
      ],
    },
    {
      title: '메뉴',
      shortcuts: [
        { keys: [mod, 'K'], label: '빠른 검색' },
        { keys: [mod, 'J'], label: '노트' },
        { keys: ['/'], label: '슬래시 메뉴' },
        { keys: ['Esc'], label: '메뉴 닫기' },
      ],
    },
    {
      title: '레이아웃',
      shortcuts: [{ keys: [mod, shift, 'M'], label: '집중 모드' }],
    },
  ];
</script>

{#snippet category(cat: Category)}
  <div>
    <h3 class={css({ fontSize: '12px', fontWeight: 'semibold', color: 'text.faint', marginBottom: '8px' })}>{cat.title}</h3>

    <div class={flex({ direction: 'column', gap: '2px' })}>
      {#each cat.shortcuts as shortcut (shortcut.label)}
        <div
          class={flex({
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '12px',
            paddingY: '4px',
          })}
        >
          <span class={css({ fontSize: '13px', color: 'text.default', whiteSpace: 'nowrap' })}>{shortcut.label}</span>
          <div class={flex({ alignItems: 'center', gap: '3px', flexShrink: '0' })}>
            {#each shortcut.keys as key, i (i)}
              <kbd
                class={css({
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  minWidth: '22px',
                  paddingX: '5px',
                  paddingY: '4px',
                  fontSize: '11px',
                  lineHeight: '[1]',
                  color: 'text.subtle',
                  borderWidth: '1px',
                  borderColor: 'border.subtle',
                  borderRadius: '4px',
                  backgroundColor: 'surface.subtle',
                })}
              >
                {#if typeof key === 'string'}
                  {key}
                {:else}
                  <Icon icon={key.icon} size={12} />
                {/if}
              </kbd>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </div>
{/snippet}

<Modal
  style={css.raw({ maxWidth: '560px', padding: '0' })}
  onclose={() => {
    app.state.shortcutsOpen = false;
  }}
  open={app.state.shortcutsOpen}
>
  <div class={css({ padding: '24px' })}>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '20px' })}>단축키</h2>

    <div class={flex({ gap: '32px' })}>
      <div class={flex({ direction: 'column', gap: '20px', flex: '1' })}>
        {#each left as cat (cat.title)}
          {@render category(cat)}
        {/each}
      </div>

      <div class={flex({ direction: 'column', gap: '20px', flex: '1' })}>
        {#each right as cat (cat.title)}
          {@render category(cat)}
        {/each}
      </div>
    </div>
  </div>
</Modal>
