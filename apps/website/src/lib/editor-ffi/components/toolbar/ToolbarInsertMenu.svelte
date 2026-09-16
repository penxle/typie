<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getAppContext } from '@typie/ui/context';
  import ChevronsDownUpIcon from '~icons/lucide/chevrons-down-up';
  import FilePlusIcon from '~icons/lucide/file-plus';
  import FileUpIcon from '~icons/lucide/file-up';
  import GalleryVerticalEndIcon from '~icons/lucide/gallery-vertical-end';
  import ImageIcon from '~icons/lucide/image';
  import ListIcon from '~icons/lucide/list';
  import ListOrderedIcon from '~icons/lucide/list-ordered';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import PlusIcon from '~icons/lucide/plus';
  import QuoteIcon from '~icons/lucide/quote';
  import TableIcon from '~icons/lucide/table';
  import HorizontalRuleIcon from '~icons/typie/horizontal-rule';
  import { blockquoteVariants, horizontalRuleVariants } from '$lib/editor-ffi/components/values';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { createHorizontalRuleVariantMessage } from '$lib/editor-ffi/handlers/variant-flow';
  import TableSizeSelector from './TableSizeSelector.svelte';
  import { toolbarRecent } from './toolbar-recent.svelte';
  import ToolbarIcon from './ToolbarIcon.svelte';
  import ToolbarPanel from './ToolbarPanel.svelte';
  import ToolbarPanelDropdown from './ToolbarPanelDropdown.svelte';
  import type { BlockquoteVariant, Fragment, HorizontalRuleVariant, Message } from '@typie/editor-ffi/browser';
  import type { Component } from 'svelte';
  import type { ToolbarPanelItem } from './ToolbarPanel.svelte';

  type Props = {
    disabled?: boolean;
  };

  let { disabled = false }: Props = $props();

  type Kind = 'image' | 'file' | 'embed' | 'hr' | 'quote' | 'callout' | 'fold' | 'table' | 'bullet' | 'ordered' | 'page_break';
  type View = 'root' | 'hr' | 'quote' | 'table';

  const app = getAppContext();
  const ctx = getEditorContext();
  const recent = toolbarRecent(app.userId);

  const layoutMode = $derived(ctx.editor?.rootAttrs?.layout_mode);
  const blockState = $derived(ctx.editor?.blockState);

  const catalog: { kind: Kind; label: string; icon: Component; keywords: string[] }[] = [
    { kind: 'image', label: '이미지', icon: ImageIcon, keywords: ['image'] },
    { kind: 'file', label: '파일', icon: PaperclipIcon, keywords: ['file'] },
    { kind: 'embed', label: '임베드', icon: FileUpIcon, keywords: ['embed'] },
    { kind: 'hr', label: '구분선', icon: HorizontalRuleIcon, keywords: ['divider'] },
    { kind: 'quote', label: '인용구', icon: QuoteIcon, keywords: ['quote'] },
    { kind: 'callout', label: '강조', icon: GalleryVerticalEndIcon, keywords: ['callout'] },
    { kind: 'fold', label: '접기', icon: ChevronsDownUpIcon, keywords: ['fold'] },
    { kind: 'table', label: '표', icon: TableIcon, keywords: ['table'] },
    { kind: 'bullet', label: '글머리 목록', icon: ListIcon, keywords: ['list'] },
    { kind: 'ordered', label: '번호 목록', icon: ListOrderedIcon, keywords: ['list'] },
    { kind: 'page_break', label: '페이지 나누기', icon: FilePlusIcon, keywords: ['page'] },
  ];

  const titles: Record<Exclude<View, 'root'>, string> = { hr: '구분선', quote: '인용구', table: '표' };

  let view = $state<View>('root');

  const noop = () => {
    return;
  };

  const rootItems = $derived<ToolbarPanelItem[]>(
    catalog
      .filter((entry) => entry.kind !== 'page_break' || layoutMode?.type === 'paginated')
      .map(({ kind, label, icon, keywords }) => ({
        id: kind,
        label,
        icon,
        keywords,
        drill: kind === 'hr' || kind === 'quote' || kind === 'table',
      })),
  );

  const insertFragment = (fragment: Fragment): Message => ({ type: 'insertion', op: { type: 'fragment', fragment } });

  const send = (message: Message, close: () => void) => {
    ctx.editor?.enqueue(message);
    ctx.editor?.focus();
    close();
  };

  const pickRoot = (id: string, close: () => void) => {
    const kind = id as Kind;
    recent.remember('insert', kind);
    if (kind === 'hr' || kind === 'quote' || kind === 'table') {
      view = kind;
      return;
    }
    const message: Message | undefined =
      kind === 'image'
        ? insertFragment({ node: { type: 'image', id: undefined } })
        : kind === 'file'
          ? insertFragment({ node: { type: 'file', id: undefined } })
          : kind === 'embed'
            ? insertFragment({ node: { type: 'embed', id: undefined } })
            : kind === 'callout'
              ? { type: 'block', op: { type: 'toggle_callout' } }
              : kind === 'fold'
                ? { type: 'block', op: { type: 'wrap_fold' } }
                : kind === 'bullet'
                  ? { type: 'list', op: { type: 'toggle_kind', kind: 'bullet' } }
                  : kind === 'ordered'
                    ? { type: 'list', op: { type: 'toggle_kind', kind: 'ordered' } }
                    : kind === 'page_break'
                      ? { type: 'insertion', op: { type: 'break', kind: 'page' } }
                      : undefined;
    if (message) send(message, close);
  };
</script>

<ToolbarPanelDropdown
  style={css.raw({ width: 'fit', paddingX: '6px', gap: '4px' })}
  {disabled}
  label="블록 삽입"
  onclose={() => (view = 'root')}
>
  {#snippet anchor()}
    <ToolbarIcon icon={PlusIcon} />
    <span class={css({ fontSize: '13px', fontWeight: 'medium' })}>삽입</span>
  {/snippet}

  {#snippet panel({ close })}
    {#key view}
      {#if view === 'root'}
        <ToolbarPanel items={rootItems} onselect={(id) => pickRoot(id, close)} placeholder="블록 삽입" recentIds={recent.ids('insert')} />
      {:else if view === 'table'}
        <ToolbarPanel items={[]} onback={() => (view = 'root')} onselect={noop} placeholder="" title={titles.table}>
          {#snippet extra()}
            <TableSizeSelector
              onSelect={(rows, cols) => {
                view = 'root';
                send({ type: 'insertion', op: { type: 'table', rows, cols } }, close);
              }}
            />
          {/snippet}
        </ToolbarPanel>
      {:else if view === 'hr'}
        <ToolbarPanel
          items={horizontalRuleVariants.map(({ variant }) => ({ id: variant, label: variant }))}
          onback={() => (view = 'root')}
          onselect={(id) => {
            view = 'root';
            send(createHorizontalRuleVariantMessage(blockState, id as HorizontalRuleVariant), close);
          }}
          placeholder=""
          rowHeight={36}
          rowJustify="center"
          title={titles.hr}
        >
          {#snippet render(item)}
            {@const Variant = horizontalRuleVariants.find((v) => v.variant === item.id)?.component}
            {#if Variant}
              <Variant />
            {/if}
          {/snippet}
        </ToolbarPanel>
      {:else}
        <ToolbarPanel
          items={blockquoteVariants.map(({ variant }) => ({ id: variant, label: variant }))}
          onback={() => (view = 'root')}
          onselect={(id) => {
            view = 'root';
            send({ type: 'block', op: { type: 'toggle_blockquote', variant: id as BlockquoteVariant } }, close);
          }}
          placeholder=""
          rowHeight={48}
          rowPaddingX="16px"
          rowPaddingY="8px"
          title={titles.quote}
        >
          {#snippet render(item)}
            {@const Variant = blockquoteVariants.find((v) => v.variant === item.id)?.component}
            {#if Variant}
              <Variant />
            {/if}
          {/snippet}
        </ToolbarPanel>
      {/if}
    {/key}
  {/snippet}
</ToolbarPanelDropdown>
