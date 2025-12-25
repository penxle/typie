<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { clamp } from '@typie/ui/utils';
  import ConstructionIcon from '~icons/lucide/construction';
  import { fragment, graphql } from '$graphql';
  import { getViewContext } from '../@split-view/context.svelte';
  import DocumentPanelInfo from './DocumentPanelInfo.svelte';
  import DocumentPanelNote from './DocumentPanelNote.svelte';
  import DocumentPanelSettings from './DocumentPanelSettings.svelte';
  import type { DocumentPanel_document } from '$graphql';
  import type { Editor } from '$lib/editor/editor.svelte';

  type Props = {
    editor: Editor;
    $document: DocumentPanel_document;
  };

  const minWidth = 240;
  const maxWidth = 400;

  let { editor, $document: _document }: Props = $props();

  const document = fragment(
    _document,
    graphql(`
      fragment DocumentPanel_document on Document {
        id

        entity {
          id
          ...DocumentPanel_Note_entity
        }

        ...DocumentPanel_Info_document
      }
    `),
  );

  const app = getAppContext();

  const splitViewId = getViewContext().id;

  const isExpanded = $derived(
    Boolean(app.preference.current.panelExpandedByViewId[splitViewId] && app.preference.current.panelTabByViewId[splitViewId]),
  );

  type Resizer = {
    deltaX: number;
    eligible: boolean;
    event: PointerEvent;
    element: HTMLElement;
  };

  let resizer = $state<Resizer | null>(null);
  let newWidth = $derived(clamp((app.preference.current.panelWidth ?? minWidth) + (resizer?.deltaX ?? 0), minWidth, maxWidth));
</script>

<aside
  style:--min-width={`${minWidth}px`}
  style:--width={`${newWidth}px`}
  style:--max-width={`${maxWidth}px`}
  class={flex({
    position: 'relative',
    zIndex: 'panel',
    backgroundColor: 'surface.default',
    flexDirection: 'column',
    flexShrink: '0',
    minWidth: isExpanded ? 'var(--min-width)' : '0',
    width: isExpanded ? 'var(--width)' : '0',
    maxWidth: isExpanded ? 'var(--max-width)' : '0',
    opacity: isExpanded ? '100' : '0',
    transitionProperty: '[min-width, max-width, opacity]',
    transitionDuration: '200ms',
    transitionTimingFunction: 'ease',
    willChange: 'min-width, max-width, opacity',
    overflow: 'hidden',
    borderLeftWidth: '1px',
    borderColor: 'border.subtle',
  })}
>
  <div
    class={css({
      position: 'absolute',
      zIndex: '1',
      top: '0',
      left: '0',
      width: '8px',
      height: 'full',
      cursor: 'col-resize',
      _hoverAfter: {
        content: '""',
        display: 'block',
        borderRightRadius: '4px',
        height: 'full',
        width: '2px',
        backgroundColor: 'border.strong',
        opacity: '50',
      },
    })}
    onpointerdowncapture={(e) => {
      resizer = {
        element: e.currentTarget,
        event: e,
        deltaX: 0,
        eligible: false,
      };
    }}
    onpointermovecapture={(e) => {
      if (!resizer) return;

      if (!resizer.eligible) {
        resizer.eligible = true;
        resizer.element.setPointerCapture(e.pointerId);
      }

      resizer.deltaX = Math.round(resizer.event.clientX - e.clientX);
    }}
    onpointerupcapture={() => {
      if (!resizer) return;

      if (resizer.eligible && resizer.element.hasPointerCapture(resizer.event.pointerId)) {
        resizer.element.releasePointerCapture(resizer.event.pointerId);
      }

      app.preference.current.panelWidth = newWidth;

      resizer = null;
    }}
  ></div>

  {#if isExpanded}
    {#if app.preference.current.panelTabByViewId[splitViewId] === 'settings'}
      <DocumentPanelSettings {editor} />
    {:else if app.preference.current.panelTabByViewId[splitViewId] === 'info'}
      <DocumentPanelInfo {$document} {editor} />
    {:else if app.preference.current.panelTabByViewId[splitViewId] === 'note'}
      <DocumentPanelNote $entity={$document.entity} />
    {:else}
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '8px',
          height: 'full',
          color: 'text.faint',
          fontSize: '12px',
        })}
      >
        <Icon style={css.raw({ color: 'text.faint' })} icon={ConstructionIcon} size={24} />
        아직 준비중인 기능이에요
      </div>
    {/if}
  {/if}
</aside>
