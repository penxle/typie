<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon, VerticalDivider } from '@typie/ui/components';
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import Minimize2Icon from '~icons/lucide/minimize-2';
  import XIcon from '~icons/lucide/x';
  import { getZenMode } from '../../zen-mode.svelte';
  import CloseButton from './CloseButton.svelte';
  import { getZenModePaneChrome } from './zen-mode-pane-chrome.svelte';
  import type { Snippet } from 'svelte';

  type Props = { menu?: Snippet; close?: boolean };
  let { menu, close = true }: Props = $props();

  const zenMode = getZenMode();
  const paneChrome = getZenModePaneChrome();
  const shortcut = $derived(zenMode.active ? (['Esc'] as ['Esc']) : (['Mod', 'Shift', 'M'] as ['Mod', 'Shift', 'M']));
</script>

<VerticalDivider style={css.raw({ height: '12px' })} />

{#if menu}
  {@render menu()}
{/if}

<button
  class={center({
    borderRadius: '4px',
    size: '24px',
    color: 'text.faint',
    transition: 'common',
    _hover: { color: 'text.subtle', backgroundColor: 'surface.muted' },
  })}
  aria-label={zenMode.active ? '집중 모드 끄기' : '집중 모드 켜기'}
  aria-pressed={zenMode.active}
  data-editor-focus-mode-control
  onclick={(event) => {
    paneChrome.prepareEntryReveal('actions', event);
    void zenMode.toggle('pane_header');
  }}
  onpointerdown={(event) => event.preventDefault()}
  type="button"
  use:tooltip={{ message: zenMode.active ? '집중 모드 끄기' : '집중 모드 켜기', keys: shortcut }}
>
  <Icon icon={zenMode.active ? Minimize2Icon : Maximize2Icon} size={16} />
</button>

{#if close}
  <CloseButton>
    <Icon icon={XIcon} size={16} />
  </CloseButton>
{/if}
