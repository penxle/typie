<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import ArrowBigUpIcon from '~icons/lucide/arrow-big-up';
  import CommandIcon from '~icons/lucide/command';
  import OptionIcon from '~icons/lucide/option';
  import { pushState } from '$app/navigation';
  import { shortcutCategories } from './shortcut-catalog';
  import type { Component } from 'svelte';
  import type { ShortcutKey, ShortcutSequence } from './shortcut-catalog';

  const app = getAppContext();

  const isMac = typeof window !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.userAgent);
  const platform = isMac ? 'mac' : 'windows';

  type Key = string | { icon: Component };
  type CheatsheetShortcut = { id: string; label: string; sequences: ShortcutSequence[] };
  type CheatsheetCategory = {
    id: string;
    title: string;
    column: 'left' | 'right';
    shortcuts: CheatsheetShortcut[];
  };

  const availableSequences = (sequences: ShortcutSequence[]): ShortcutSequence[] =>
    sequences.filter((sequence) => sequence.platforms === undefined || sequence.platforms.includes(platform));

  const displayKey = (key: ShortcutKey): Key => {
    if (key === 'mod') return isMac ? { icon: CommandIcon } : 'Ctrl';
    if (key === 'alt') return isMac ? { icon: OptionIcon } : 'Alt';
    if (key === 'shift') return isMac ? { icon: ArrowBigUpIcon } : 'Shift';
    return key;
  };

  const cheatsheetCategories: CheatsheetCategory[] = shortcutCategories.flatMap((category) => {
    const column = category.cheatsheetColumn;
    if (column === undefined) return [];

    const shortcuts = category.shortcuts.flatMap((shortcut) => {
      if (shortcut.cheatsheet === undefined) return [];

      return [
        {
          id: shortcut.id,
          label: shortcut.cheatsheet.label ?? shortcut.label,
          sequences: availableSequences(shortcut.cheatsheet.sequences ?? shortcut.sequences),
        },
      ];
    });

    return [{ id: category.id, title: category.title, column, shortcuts }];
  });

  const left = cheatsheetCategories.filter((category) => category.column === 'left');
  const right = cheatsheetCategories.filter((category) => category.column === 'right');

  let openSettingsAfterClose = false;

  const openShortcutSettings = () => {
    openSettingsAfterClose = true;
    app.state.shortcutsOpen = false;
  };

  const handleTransitionEnd = () => {
    if (!openSettingsAfterClose) return;
    openSettingsAfterClose = false;
    pushState('', { shallowRoute: '/preference/shortcuts' });
  };
</script>

{#snippet category(cat: CheatsheetCategory)}
  <div>
    <h3 class={css({ fontSize: '12px', fontWeight: 'semibold', color: 'text.faint', marginBottom: '8px' })}>{cat.title}</h3>

    <div class={flex({ direction: 'column', gap: '2px' })}>
      {#each cat.shortcuts as shortcut (shortcut.id)}
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
            {#each shortcut.sequences as sequence, sequenceIndex (sequenceIndex)}
              {#if sequenceIndex > 0}
                <span class={css({ paddingX: '2px', fontSize: '11px', color: 'text.faint' })}>/</span>
              {/if}
              <div class={flex({ alignItems: 'center', gap: '3px' })}>
                {#each sequence.keys as key, keyIndex (keyIndex)}
                  {@const displayedKey = displayKey(key)}
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
                    {#if typeof displayedKey === 'string'}
                      {displayedKey}
                    {:else}
                      <Icon icon={displayedKey.icon} size={12} />
                    {/if}
                  </kbd>
                {/each}
              </div>
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
  ontransitionend={handleTransitionEnd}
  open={app.state.shortcutsOpen}
>
  <div class={css({ padding: '24px' })}>
    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', marginBottom: '24px' })}>
      <h2 class={css({ fontSize: '15px', fontWeight: 'bold', letterSpacing: '-0.01em', color: 'text.default' })}>단축키</h2>
      <button
        class={css({
          padding: '0',
          fontSize: '12px',
          color: 'text.subtle',
          borderWidth: '0',
          backgroundColor: 'transparent',
          cursor: 'pointer',
          _hover: { color: 'text.default', textDecoration: 'underline', textUnderlineOffset: '2px' },
        })}
        onclick={openShortcutSettings}
        type="button"
      >
        더 보기…
      </button>
    </div>

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
