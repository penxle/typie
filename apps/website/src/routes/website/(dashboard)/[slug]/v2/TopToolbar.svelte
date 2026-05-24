<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon, Menu } from '@typie/ui/components';
  import ChevronsDownUpIcon from '~icons/lucide/chevrons-down-up';
  import FilePlusIcon from '~icons/lucide/file-plus';
  import FileUpIcon from '~icons/lucide/file-up';
  import GalleryVerticalEndIcon from '~icons/lucide/gallery-vertical-end';
  import ImageIcon from '~icons/lucide/image';
  import ListIcon from '~icons/lucide/list';
  import ListOrderedIcon from '~icons/lucide/list-ordered';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import QuoteIcon from '~icons/lucide/quote';
  import ScissorsIcon from '~icons/lucide/scissors';
  import TableIcon from '~icons/lucide/table';
  import TableSizeSelector from '$lib/components/editor/toolbar/TableSizeSelector.svelte';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import ToolbarButton from './ToolbarButton.svelte';
  import type { BlockquoteVariant, CalloutVariant, Fragment, HorizontalRuleVariant, Message } from '@typie/editor-ffi/browser';

  const ctx = getEditorContext();

  const enqueue = (message: Message) => {
    ctx.editor?.enqueue(message);
    ctx.editor?.focus();
  };

  const insertFragment = (fragment: Fragment): Message => ({
    type: 'insertion',
    op: { type: 'fragment', fragment },
  });

  const horizontalRuleVariants: { label: string; value: HorizontalRuleVariant }[] = [
    { label: '실선', value: 'line' },
    { label: '점선', value: 'dashed_line' },
    { label: '원 라인', value: 'circle_line' },
    { label: '다이아 라인', value: 'diamond_line' },
    { label: '원', value: 'circle' },
    { label: '다이아', value: 'diamond' },
    { label: '원 3개', value: 'three_circles' },
    { label: '다이아 3개', value: 'three_diamonds' },
    { label: '지그재그', value: 'zigzag' },
  ];

  const blockquoteVariants: { label: string; value: BlockquoteVariant }[] = [
    { label: '왼쪽 선', value: 'left_line' },
    { label: '왼쪽 따옴표', value: 'left_quote' },
    { label: '보낸 메시지', value: 'message_sent' },
    { label: '받은 메시지', value: 'message_received' },
  ];

  const calloutVariants: { label: string; value: CalloutVariant }[] = [
    { label: '정보', value: 'info' },
    { label: '성공', value: 'success' },
    { label: '경고', value: 'warning' },
    { label: '위험', value: 'danger' },
  ];

  let selectedHrVariant = $state<HorizontalRuleVariant>('line');
  let selectedBqVariant = $state<BlockquoteVariant>('left_line');
  let selectedCalloutVariant = $state<CalloutVariant>('info');
</script>

<div
  class={css({
    display: 'flex',
    flexShrink: '0',
    alignItems: 'center',
    gap: '6px',
    paddingX: '12px',
    paddingY: '6px',
    borderBottomWidth: '1px',
    borderColor: 'border.subtle',
    backgroundColor: 'surface.default',
    flexWrap: 'wrap',
  })}
  role="toolbar"
>
  <ToolbarButton icon={ImageIcon} label="이미지" onclick={() => enqueue(insertFragment({ node: { type: 'image', id: undefined } }))} />
  <ToolbarButton icon={PaperclipIcon} label="파일" onclick={() => enqueue(insertFragment({ node: { type: 'file', id: undefined } }))} />
  <ToolbarButton icon={FileUpIcon} label="임베드" onclick={() => enqueue(insertFragment({ node: { type: 'embed', id: undefined } }))} />

  <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>

  <div class={css({ display: 'flex', alignItems: 'center', gap: '2px' })}>
    <select
      class={css({
        fontSize: '12px',
        paddingX: '4px',
        paddingY: '2px',
        borderRadius: '4px',
        borderWidth: '1px',
        borderColor: 'border.subtle',
      })}
      onchange={(e) => {
        selectedHrVariant = e.currentTarget.value as HorizontalRuleVariant;
      }}
    >
      {#each horizontalRuleVariants as { label, value } (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
    <ToolbarButton
      icon={ScissorsIcon}
      label="구분선 삽입"
      onclick={() => enqueue(insertFragment({ node: { type: 'horizontal_rule', variant: selectedHrVariant } }))}
    />
  </div>

  <div class={css({ display: 'flex', alignItems: 'center', gap: '2px' })}>
    <select
      class={css({
        fontSize: '12px',
        paddingX: '4px',
        paddingY: '2px',
        borderRadius: '4px',
        borderWidth: '1px',
        borderColor: 'border.subtle',
      })}
      onchange={(e) => {
        selectedBqVariant = e.currentTarget.value as BlockquoteVariant;
      }}
    >
      {#each blockquoteVariants as { label, value } (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
    <ToolbarButton
      icon={QuoteIcon}
      label="인용구 삽입"
      onclick={() => enqueue(insertFragment({ node: { type: 'blockquote', variant: selectedBqVariant } }))}
    />
  </div>

  <div class={css({ display: 'flex', alignItems: 'center', gap: '2px' })}>
    <select
      class={css({
        fontSize: '12px',
        paddingX: '4px',
        paddingY: '2px',
        borderRadius: '4px',
        borderWidth: '1px',
        borderColor: 'border.subtle',
      })}
      onchange={(e) => {
        selectedCalloutVariant = e.currentTarget.value as CalloutVariant;
      }}
    >
      {#each calloutVariants as { label, value } (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
    <ToolbarButton
      icon={GalleryVerticalEndIcon}
      label="강조 삽입"
      onclick={() => enqueue(insertFragment({ node: { type: 'callout', variant: selectedCalloutVariant } }))}
    />
  </div>

  <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>

  <ToolbarButton icon={ChevronsDownUpIcon} label="접기" onclick={() => enqueue(insertFragment({ node: { type: 'fold' } }))} />
  <Menu
    style={{
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      size: '28px',
      borderRadius: '4px',
      borderWidth: '1px',
      borderColor: 'transparent',
      cursor: 'pointer',
      color: 'text.subtle',
      _hover: { backgroundColor: 'surface.muted' },
    }}
    offset={4}
    placement="bottom-start"
  >
    {#snippet button()}
      <Icon icon={TableIcon} size={16} />
    {/snippet}
    {#snippet children({ close })}
      <TableSizeSelector
        onSelect={(rows, cols) => {
          close();
          enqueue(
            insertFragment({
              node: { type: 'table' },
              children: Array.from({ length: rows }, () => ({
                node: { type: 'table_row' },
                children: Array.from({ length: cols }, () => ({
                  node: { type: 'table_cell', col_width: undefined, background_color: undefined },
                  children: [{ node: { type: 'paragraph' } }],
                })),
              })),
            }),
          );
        }}
      />
    {/snippet}
  </Menu>
  <ToolbarButton icon={ListIcon} label="순서 없는 목록" onclick={() => enqueue(insertFragment({ node: { type: 'bullet_list' } }))} />
  <ToolbarButton
    icon={ListOrderedIcon}
    label="순서 있는 목록"
    onclick={() => enqueue(insertFragment({ node: { type: 'ordered_list' } }))}
  />

  <div class={css({ width: '1px', height: '16px', backgroundColor: 'border.subtle' })}></div>

  <ToolbarButton
    icon={FilePlusIcon}
    label="페이지 나누기"
    onclick={() => enqueue({ type: 'insertion', op: { type: 'break', kind: 'page' } })}
  />
</div>
