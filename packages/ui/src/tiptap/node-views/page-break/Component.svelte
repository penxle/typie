<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import FileDownIcon from '~icons/lucide/file-down';
  import { Icon } from '../../../components';
  import { mmToPx } from '../../../utils';
  import { NodeView } from '../../lib';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { editor, HTMLAttributes, getPos }: Props = $props();

  const pageLayout = $derived(editor?.current.storage?.page?.layout);
  const hasPageLayout = $derived(!!pageLayout);

  let pageBreakEl = $state<HTMLElement>();

  const isInSelection = $derived.by(() => {
    if (!editor?.current) return false;
    const pos = getPos();
    if (pos === undefined) return false;

    const { from, to } = editor.current.state.selection;
    const nodeSize = editor.current.state.doc.nodeAt(pos)?.nodeSize || 0;

    return pos >= from && pos + nodeSize <= to;
  });

  const calculateRemainingHeight = () => {
    if (!hasPageLayout || !pageLayout || !pageBreakEl || !editor) return 0;

    const editorView = editor.current?.view;
    if (!editorView) return 0;

    const pos = getPos();
    if (pos === undefined) return 0;

    const coords = editorView.coordsAtPos(pos);

    const pageBreaksContainer = editorView.dom.querySelector('[data-page-break="true"]');
    if (!pageBreaksContainer) return 0;

    const breakers = pageBreaksContainer.querySelectorAll('.breaker');
    for (const breaker of breakers) {
      const breakerRect = (breaker as HTMLElement).getBoundingClientRect();
      if (breakerRect.top > coords.top) {
        return Math.max(0, breakerRect.top - coords.bottom);
      }
    }

    const { height, marginBottom } = pageLayout;
    const contentHeight = mmToPx(height - marginBottom);
    return contentHeight;
  };

  $effect(() => {
    if (pageBreakEl) {
      if (hasPageLayout) {
        void getPos();

        const height = calculateRemainingHeight();
        pageBreakEl.style.height = `${height}px`;
      } else {
        pageBreakEl.style.height = '20px';
      }
    }
  });
</script>

<NodeView
  class={cx(
    'page-break-node',
    css({
      width: 'full',
      '[data-layout="page"] .page-break-node+&, [data-layout="page"] .page-break-node+.selected-node &': {
        marginTop:
          '[calc(var(--prosemirror-page-margin-bottom) + var(--prosemirror-page-gap-height) + var(--prosemirror-page-margin-top))]',
      },
      opacity: isInSelection ? '100' : '0',
      '.selected-node &': {
        opacity: '100',
      },
      pageBreakAfter: 'always',
    }),
  )}
  {...HTMLAttributes}
  contentEditable={false}
>
  <div bind:this={pageBreakEl} class={css({ width: 'full' })}>
    <div class={flex({ alignItems: 'center', gap: '6px', color: 'accent.brand.default' })}>
      <Icon icon={FileDownIcon} size={16} />
      <span class={css({ fontSize: '12px', fontWeight: 'medium' })}>페이지 나누기</span>
    </div>
  </div>
</NodeView>
