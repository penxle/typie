<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { DropdownMenu, DropdownMenuItem, VerticalDivider } from '@typie/ui/components';
  import ChevronsDownUpIcon from '~icons/lucide/chevrons-down-up';
  import FilePlusIcon from '~icons/lucide/file-plus';
  import FileUpIcon from '~icons/lucide/file-up';
  import GalleryVerticalEndIcon from '~icons/lucide/gallery-vertical-end';
  import ImageIcon from '~icons/lucide/image';
  import ListIcon from '~icons/lucide/list';
  import ListOrderedIcon from '~icons/lucide/list-ordered';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import QuoteIcon from '~icons/lucide/quote';
  import TableIcon from '~icons/lucide/table';
  import HorizontalRuleIcon from '~icons/typie/horizontal-rule';
  import { blockquoteVariants, horizontalRuleVariants } from '$lib/editor-ffi/components/values';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { createHorizontalRuleVariantMessage } from '$lib/editor-ffi/handlers/variant-flow';
  import TableSizeSelector from './TableSizeSelector.svelte';
  import ToolbarButton from './ToolbarButton.svelte';
  import ToolbarDropdownButton from './ToolbarDropdownButton.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import type { Fragment, Message } from '@typie/editor-ffi/browser';

  const ctx = getEditorContext();

  const layoutMode = $derived(ctx.editor?.rootAttrs?.layout_mode);
  const blockState = $derived(ctx.editor?.blockState);
  const editingDisabled = $derived(ctx.editor?.terminal === true || (ctx.editor !== undefined && ctx.editor !== ctx.liveEditor));

  const enqueue = (message: Message) => {
    if (editingDisabled) return;
    ctx.editor?.enqueue(message);
    ctx.editor?.focus();
  };

  const insertFragment = (fragment: Fragment): Message => ({
    type: 'insertion',
    op: { type: 'fragment', fragment },
  });
</script>

<div
  class={flex({
    alignItems: 'center',
    gap: '4px',
    opacity: editingDisabled ? '40' : '100',
    pointerEvents: editingDisabled ? 'none' : 'auto',
  })}
>
  <ToolbarButton icon={ImageIcon} label="이미지" onclick={() => enqueue(insertFragment({ node: { type: 'image', id: undefined } }))} />

  <ToolbarButton icon={PaperclipIcon} label="파일" onclick={() => enqueue(insertFragment({ node: { type: 'file', id: undefined } }))} />

  <ToolbarButton icon={FileUpIcon} label="임베드" onclick={() => enqueue(insertFragment({ node: { type: 'embed', id: undefined } }))} />

  <ToolbarDropdownButton label="구분선">
    {#snippet anchor()}
      <ToolbarIcon icon={HorizontalRuleIcon} />
    {/snippet}

    {#snippet floating({ close })}
      <DropdownMenu style={css.raw({ maxWidth: '200px' })}>
        {#each horizontalRuleVariants as { variant, component: Component } (variant)}
          <DropdownMenuItem
            style={css.raw({ justifyContent: 'center', height: '48px' })}
            onclick={() => {
              enqueue(createHorizontalRuleVariantMessage(blockState, variant));
              close();
            }}
          >
            <Component />
          </DropdownMenuItem>
        {/each}
      </DropdownMenu>
    {/snippet}
  </ToolbarDropdownButton>

  <ToolbarDropdownButton label="인용구">
    {#snippet anchor()}
      <ToolbarIcon icon={QuoteIcon} />
    {/snippet}

    {#snippet floating({ close })}
      <DropdownMenu style={css.raw({ maxWidth: '200px' })}>
        {#each blockquoteVariants as { variant, component: Component } (variant)}
          <DropdownMenuItem
            style={css.raw({ height: '48px' })}
            onclick={() => {
              enqueue({ type: 'block', op: { type: 'toggle_blockquote', variant } });
              close();
            }}
          >
            <Component />
          </DropdownMenuItem>
        {/each}
      </DropdownMenu>
    {/snippet}
  </ToolbarDropdownButton>

  <ToolbarButton icon={GalleryVerticalEndIcon} label="강조" onclick={() => enqueue({ type: 'block', op: { type: 'toggle_callout' } })} />

  <ToolbarButton icon={ChevronsDownUpIcon} label="접기" onclick={() => enqueue({ type: 'block', op: { type: 'wrap_fold' } })} />

  <ToolbarDropdownButton label="표" placement="bottom-start">
    {#snippet anchor()}
      <ToolbarIcon icon={TableIcon} />
    {/snippet}

    {#snippet floating({ close })}
      <TableSizeSelector
        onSelect={(rows, cols) => {
          enqueue({ type: 'insertion', op: { type: 'table', rows, cols } });
          close();
        }}
      />
    {/snippet}
  </ToolbarDropdownButton>

  <ToolbarDropdownButton label="목록">
    {#snippet anchor()}
      <ToolbarIcon icon={ListIcon} />
    {/snippet}

    {#snippet floating({ close })}
      <DropdownMenu>
        <DropdownMenuItem
          onclick={() => {
            enqueue({ type: 'list', op: { type: 'toggle_kind', kind: 'bullet' } });
            close();
          }}
        >
          <div class={flex({ alignItems: 'center', gap: '4px' })}>
            <ToolbarIcon icon={ListIcon} />
            순서 없는 목록
          </div>
        </DropdownMenuItem>

        <DropdownMenuItem
          onclick={() => {
            enqueue({ type: 'list', op: { type: 'toggle_kind', kind: 'ordered' } });
            close();
          }}
        >
          <div class={flex({ alignItems: 'center', gap: '4px' })}>
            <ToolbarIcon icon={ListOrderedIcon} />
            순서 있는 목록
          </div>
        </DropdownMenuItem>
      </DropdownMenu>
    {/snippet}
  </ToolbarDropdownButton>

  {#if layoutMode?.type === 'paginated'}
    <VerticalDivider style={css.raw({ height: '12px' })} />

    <ToolbarButton
      icon={FilePlusIcon}
      label="페이지 나누기"
      onclick={() => enqueue({ type: 'insertion', op: { type: 'break', kind: 'page' } })}
    />
  {/if}
</div>
