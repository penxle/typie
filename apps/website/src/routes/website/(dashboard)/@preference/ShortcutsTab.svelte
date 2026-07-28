<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { SettingsCard, SettingsDivider, SettingsRow } from '$lib/components';
  import { graphql } from '$mearie';
  import { shortcutCategories } from '../shortcut-catalog';
  import type { DashboardLayout_PreferenceModal_ShortcutsTab_user$key } from '$mearie';
  import type { ShortcutKey, ShortcutPlatform, ShortcutSequence } from '../shortcut-catalog';

  type Props = {
    user$key: DashboardLayout_PreferenceModal_ShortcutsTab_user$key;
  };

  let { user$key }: Props = $props();

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_ShortcutsTab_user on User {
        id
      }
    `),
    () => user$key,
  );

  const isMac = typeof window !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.userAgent);
  const platform: ShortcutPlatform = isMac ? 'mac' : 'windows';

  const displayKey = (key: ShortcutKey): string => {
    if (key === 'mod') return isMac ? 'Cmd' : 'Ctrl';
    if (key === 'alt') return isMac ? 'Option' : 'Alt';
    if (key === 'shift') return 'Shift';
    return key;
  };

  const availableSequences = (sequences: ShortcutSequence[]): ShortcutSequence[] =>
    sequences.filter((sequence) => sequence.platforms === undefined || sequence.platforms.includes(platform));
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
        {#each category.shortcuts as shortcut, index (shortcut.id)}
          {#if index > 0}
            <SettingsDivider />
          {/if}
          <SettingsRow>
            {#snippet label()}
              {shortcut.label}
            {/snippet}
            {#snippet value()}
              <div
                class={flex({
                  align: 'center',
                  gap: '4px',
                  flexShrink: 0,
                })}
              >
                {#each availableSequences(shortcut.sequences) as sequence, sequenceIndex (sequenceIndex)}
                  {#if sequenceIndex > 0}
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
                    {#each sequence.keys as key, keyIndex (keyIndex)}
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
                        {displayKey(key)}
                      </kbd>
                    {/each}
                  </div>
                {/each}
              </div>
            {/snippet}
          </SettingsRow>
        {/each}
      </SettingsCard>
    </div>
  {/each}
</div>
