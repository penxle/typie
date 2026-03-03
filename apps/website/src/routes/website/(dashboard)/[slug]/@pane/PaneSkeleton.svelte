<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { getAppContext } from '@typie/ui/context';
  import type { LayoutMode } from '$lib/editor/types';
  import type { Pane } from './types';

  const DEFAULT_CONTENT_WIDTH = 600;
  const DEFAULT_PARAGRAPH_TOP_PADDING = 16;
  const PAGE_HEADER_GAP = 20;

  type Props = {
    pane: Pane;
    documentLayoutMode?: LayoutMode | null;
  };

  let { pane, documentLayoutMode = null }: Props = $props();

  const app = getAppContext();
  const toolbarSize = $derived(app.preference.current.toolbarStyle === 'compact' ? 'medium' : 'large');
  const insertSize = $derived(toolbarSize === 'large' ? '48px' : '28px');
  const panelTabWidth = $derived(toolbarSize === 'large' ? '48px' : '40px');
  const panelTabHeight = $derived(toolbarSize === 'large' ? '37px' : '24px');

  const layoutMetrics = $derived.by(() => {
    if (!documentLayoutMode) {
      return {
        isPaginated: false,
        contentWidth: DEFAULT_CONTENT_WIDTH,
        paragraphTopPadding: DEFAULT_PARAGRAPH_TOP_PADDING,
      };
    }

    if (documentLayoutMode.type === 'continuous') {
      const width = Math.round(documentLayoutMode.maxWidth);
      return {
        isPaginated: false,
        contentWidth: width,
        paragraphTopPadding: DEFAULT_PARAGRAPH_TOP_PADDING,
      };
    }

    return {
      isPaginated: true,
      contentWidth: Math.round(documentLayoutMode.pageWidth - documentLayoutMode.pageMarginLeft - documentLayoutMode.pageMarginRight),
      paragraphTopPadding: Math.round(documentLayoutMode.pageMarginTop),
    };
  });

  const isPaginated = $derived(layoutMetrics.isPaginated);
  const bodyMaxWidth = $derived(`${layoutMetrics.contentWidth}px`);
  const contentMaxWidth = $derived(`${layoutMetrics.contentWidth}px`);
  const paragraphPaddingTop = $derived(`${layoutMetrics.paragraphTopPadding}px`);

  function seededRandom(seed: number) {
    return () => {
      seed = (seed * 16_807 + 0) % 2_147_483_647;
      return (seed - 1) / 2_147_483_646;
    };
  }

  const rand = seededRandom([...pane.id].reduce((a, c) => a + (c.codePointAt(0) ?? 0), 0));
  const variants = ['skeleton-typing-a', 'skeleton-typing-b', 'skeleton-typing-c'] as const;
  function generateLines(count: number) {
    return Array.from({ length: count }, () => {
      const duration = 3.5 + rand() * 1.5;
      return {
        width: `${60 + Math.floor(rand() * 36)}%`,
        animation: `pulse 2s ease-in-out infinite, ${variants[Math.floor(rand() * 3)]} ${duration}s ease-in-out ${-rand() * duration}s infinite`,
      };
    });
  }

  const linesBefore = generateLines(3);
  const linesAfter = generateLines(5);

  function unreachable(x: never): never {
    throw new Error(`unreachable: ${String(x)}`);
  }

  const bar = css({
    backgroundColor: 'surface.muted',
    borderRadius: '4px',
    animation: 'pulse 2s ease-in-out infinite',
    flexShrink: '0',
  });

  const textLine = css({
    backgroundColor: 'surface.muted',
    borderRadius: '4px',
    flexShrink: '0',
    transformOrigin: 'left',
  });

  const divider = css({
    height: '1px',
    backgroundColor: 'surface.muted',
    flexShrink: '0',
  });

  const verticalDivider = css({
    backgroundColor: 'surface.muted',
    flexShrink: '0',
  });

  const toolbarRow = flex({
    alignItems: 'center',
    flexShrink: '0',
    borderBottomWidth: '1px',
    borderColor: 'border.subtle',
  });
</script>

{#if pane.kind === 'entity'}
  <div class={flex({ flexDirection: 'column', size: 'full' })}>
    <!-- Header (36px): breadcrumb ... controls close -->
    <div
      class={flex({
        alignItems: 'center',
        justifyContent: 'space-between',
        height: '36px',
        paddingLeft: '24px',
        paddingRight: '8px',
        flexShrink: '0',
      })}
    >
      <!-- Breadcrumb (left) -->
      <div class={flex({ alignItems: 'center', gap: '4px', overflow: 'hidden' })}>
        <div style:width="12px" style:height="12px" class={bar}></div>
        <div style:width="36px" style:height="12px" class={bar}></div>
        <div style:width="12px" style:height="12px" style:border-radius="2px" class={bar}></div>
        <div style:width="60px" style:height="12px" class={bar}></div>
      </div>
      <!-- Controls (right) -->
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <!-- Feedback button -->
        <div style:width="87px" style:height="22px" style:border-radius="4px" class={bar}></div>
        <!-- Menu button -->
        <div style:width="24px" style:height="24px" style:border-radius="4px" class={bar}></div>
        <!-- Lock button -->
        <div style:width="24px" style:height="24px" style:border-radius="4px" class={bar}></div>
        <!-- Zen mode button -->
        <div style:width="24px" style:height="24px" style:border-radius="4px" class={bar}></div>
        <!-- Close button spacer -->
        <div style:width="24px" style:height="24px" style:flex-shrink="0"></div>
      </div>
    </div>

    <div class={divider}></div>

    <!-- TopToolbar: insert icons | panel tabs -->
    <div
      style:padding-left="16px"
      style:padding-right="10px"
      style:padding-top="6px"
      style:padding-bottom="6px"
      style:gap="4px"
      class={toolbarRow}
    >
      <!-- Insert buttons (image, file, embed, hr, quote, callout, fold, table, list) -->
      <div class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}>
        {#each { length: 9 }}
          <div style:width={insertSize} style:height={insertSize} class={bar}></div>
        {/each}
      </div>
      <div style:flex-grow="1"></div>
      <!-- Vertical divider (height 80%, marginX 12px) -->
      <div style:width="1px" style:height="80%" style:margin-inline="12px" class={verticalDivider}></div>
      <!-- Panel tabs: info, note, comments, spellcheck, ai, timeline, settings -->
      <div class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}>
        {#each { length: 7 }}
          <div style:width={panelTabWidth} style:height={panelTabHeight} class={bar}></div>
        {/each}
      </div>
    </div>

    <!-- BottomToolbar: undo/redo | colors/font | formatting | link/ruby | align | clear ... search -->
    <div
      style:padding-left="20px"
      style:padding-right="12px"
      style:padding-top="8px"
      style:padding-bottom="8px"
      style:gap="10px"
      class={toolbarRow}
    >
      <!-- Undo / Redo (gap 4px) -->
      <div class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}>
        <div style:width="24px" style:height="24px" class={bar}></div>
        <div style:width="24px" style:height="24px" class={bar}></div>
      </div>
      <div style:width="1px" style:height="12px" class={verticalDivider}></div>
      <!-- Text color, bg color, font family, font weight, font size (gap 4px) -->
      <div class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}>
        <div style:width="46px" style:height="24px" class={bar}></div>
        <div style:width="46px" style:height="24px" class={bar}></div>
        <div style:width="120px" style:height="24px" class={bar}></div>
        <div style:width="100px" style:height="24px" class={bar}></div>
        <div style:width="50px" style:height="24px" class={bar}></div>
      </div>
      <div style:width="1px" style:height="12px" class={verticalDivider}></div>
      <!-- Bold, Italic, Strikethrough, Underline (gap 4px) -->
      <div class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}>
        <div style:width="24px" style:height="24px" class={bar}></div>
        <div style:width="24px" style:height="24px" class={bar}></div>
        <div style:width="24px" style:height="24px" class={bar}></div>
        <div style:width="24px" style:height="24px" class={bar}></div>
      </div>
      <div style:width="1px" style:height="12px" class={verticalDivider}></div>
      <!-- Link, Ruby (gap 4px) -->
      <div class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}>
        <div style:width="24px" style:height="24px" class={bar}></div>
        <div style:width="24px" style:height="24px" class={bar}></div>
      </div>
      <div style:width="1px" style:height="12px" class={verticalDivider}></div>
      <!-- Align, Line height, Letter spacing (gap 4px) -->
      <div class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}>
        <div style:width="24px" style:height="24px" class={bar}></div>
        <div style:width="24px" style:height="24px" class={bar}></div>
        <div style:width="24px" style:height="24px" class={bar}></div>
      </div>
      <div style:width="1px" style:height="12px" class={verticalDivider}></div>
      <!-- Clear formatting -->
      <div style:width="24px" style:height="24px" class={bar}></div>
      <div style:flex-grow="1"></div>
      <!-- Search -->
      <div style:width="24px" style:height="24px" class={bar}></div>
    </div>

    <!-- Body: centered content with constrained width -->
    <div class={flex({ flexGrow: '1', overflow: 'hidden' })}>
      <div
        style:max-width={bodyMaxWidth}
        class={flex({
          flexDirection: 'column',
          width: 'full',
          marginX: 'auto',
          paddingTop: '60px',
        })}
      >
        <div style:max-width={contentMaxWidth} class={css({ width: 'full', marginX: 'auto' })}>
          <!-- Title (fontSize 28px) -->
          <div style:width="45%" style:height="1lh" style:font-size="28px" class={bar}></div>
          <!-- Subtitle (marginTop 4px, fontSize 16px) -->
          <div style:width="30%" style:height="1lh" style:font-size="16px" style:margin-top="4px" class={bar}></div>
          {#if !isPaginated}
            <!-- Divider (continuous mode only) -->
            <div style:margin-top="10px" class={divider}></div>
          {/if}
        </div>

        {#snippet paragraphs()}
          <!-- Paragraphs (fontSize 16px, lineHeight 1.6, paragraphSpacing 16px) -->
          <div
            style:font-size="16px"
            style:line-height="1.6"
            style:padding-top={paragraphPaddingTop}
            class={flex({ flexDirection: 'column', gap: '16px' })}
          >
            {#each linesBefore as line, i (i)}
              <div style:height="1lh" class={flex({ alignItems: 'center' })}>
                <div style:width={line.width} style:height="16px" style:animation={line.animation} class={textLine}></div>
              </div>
            {/each}
            <!-- Image placeholder -->
            <div
              class={css({
                backgroundColor: 'surface.muted',
                borderRadius: '8px',
                width: 'full',
                height: '320px',
                animation: 'pulse 2s ease-in-out infinite',
              })}
            ></div>
            {#each linesAfter as line, i (i)}
              <div style:height="1lh" class={flex({ alignItems: 'center' })}>
                <div style:width={line.width} style:height="16px" style:animation={line.animation} class={textLine}></div>
              </div>
            {/each}
          </div>
        {/snippet}

        {#key isPaginated}
          {#if isPaginated}
            <div
              style:max-width={contentMaxWidth}
              style:margin-top={`${PAGE_HEADER_GAP}px`}
              class={css({ width: 'full', marginX: 'auto' })}
            >
              {@render paragraphs()}
            </div>
          {:else}
            <div style:max-width={contentMaxWidth} class={css({ width: 'full', marginX: 'auto' })}>
              {@render paragraphs()}
            </div>
          {/if}
        {/key}
      </div>
    </div>
  </div>
{:else if pane.kind === 'home'}
  <!-- Home skeleton: centered like the actual HomePane layout -->
  <div class={css({ width: 'full', height: 'full', overflowY: 'auto' })}>
    <div
      class={flex({
        flexDirection: 'column',
        justifyContent: 'center',
        gap: '32px',
        width: '800px',
        maxWidth: 'full',
        minHeight: 'full',
        marginX: 'auto',
        padding: '64px',
      })}
    >
      <!-- Logo placeholder -->
      <div class={flex({ flexDirection: 'column', gap: '12px', alignItems: 'flex-start', width: 'full' })}>
        <div style:width="32px" style:height="32px" class={bar}></div>
        <!-- Greeting -->
        <div style:width="280px" style:max-width="100%" style:height="28px" class={bar}></div>
      </div>

      <!-- Recent items section -->
      <div class={flex({ flexDirection: 'column', gap: '16px', width: 'full' })}>
        <div style:width="100px" style:height="20px" class={bar}></div>
        {#each { length: 5 }}
          <div
            class={css({
              padding: '12px',
              borderRadius: '8px',
              backgroundColor: 'surface.subtle',
              animation: 'pulse 2s ease-in-out infinite',
            })}
          >
            <div class={flex({ flexDirection: 'column', gap: '4px' })}>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <div style:width="16px" style:height="16px" class={bar}></div>
                <div style:width="40%" style:height="14px" class={bar}></div>
              </div>
              <div class={css({ paddingLeft: '24px' })}>
                <div style:width="60%" style:height="12px" class={bar}></div>
              </div>
            </div>
          </div>
        {/each}
      </div>

      <!-- Recent activity section -->
      <div class={flex({ flexDirection: 'column', gap: '16px', width: 'full' })}>
        <div style:width="80px" style:height="20px" class={bar}></div>
        <div style:width="100%" style:height="120px" style:border-radius="4px" class={bar}></div>
      </div>
    </div>
  </div>
{:else}
  {unreachable(pane)}
{/if}
