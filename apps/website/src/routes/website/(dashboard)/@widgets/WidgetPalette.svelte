<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon } from '@typie/ui/components';
  import { slide } from 'svelte/transition';
  import PlusIcon from '~icons/lucide/plus';
  import { dragPaletteWidget } from './drag-palette-widget-action';
  import { WIDGET_CATEGORIES, WIDGET_COMPONENTS, WIDGET_METADATA } from './widgets';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';
  import type { Editor_Widget_CharacterCountChangeWidget_post, Editor_Widget_PostRelatedNoteWidget_post } from '$graphql';
  import type { WidgetType } from './widget-context.svelte';

  type Props = {
    open: boolean;
    editor?: Ref<Editor>;
    $post?: Editor_Widget_CharacterCountChangeWidget_post & Editor_Widget_PostRelatedNoteWidget_post;
    addedWidgets?: WidgetType[];
    onDragStart: (e: PointerEvent, widgetType: WidgetType) => void;
    onDragMove: (e: PointerEvent) => void;
    onDragEnd: (e: PointerEvent) => void;
    onDragCancel: () => void;
  };

  let {
    open = $bindable(false),
    editor,
    $post: _post,
    addedWidgets = [],
    onDragStart,
    onDragMove,
    onDragEnd,
    onDragCancel,
  }: Props = $props();

  import { getWidgetContext, setupWidgetContext } from './widget-context.svelte';

  const parentContext = getWidgetContext();
  const widgetContext = setupWidgetContext();

  widgetContext.createWidget = parentContext.createWidget;
  widgetContext.deleteWidget = parentContext.deleteWidget;
  widgetContext.updateWidget = parentContext.updateWidget;
  widgetContext.moveWidget = parentContext.moveWidget;

  $effect(() => {
    widgetContext.env.editor = editor;
    widgetContext.env.$post = _post;
    widgetContext.env.editMode = false;
    widgetContext.env.palette = true;
  });

  const widgetsByCategory = $derived(
    WIDGET_CATEGORIES.map((category) => ({
      category,
      widgets: WIDGET_METADATA.filter((w) => w.category === category.id && w.type !== 'onboarding'),
    })).filter((group) => group.widgets.length > 0),
  );

  const handleAddWidget = (widgetType: WidgetType) => {
    if (widgetType) {
      widgetContext.createWidget?.(widgetType, 0);
    }
  };
</script>

{#if open}
  <div
    class={flex({
      position: 'fixed',
      bottom: '0',
      left: '1/2',
      transform: 'translateX(-50%)',
      flexDirection: 'column',
      width: 'fit',
      maxHeight: '[60dvh]',
      backgroundColor: 'surface.default',
      borderTopLeftRadius: '16px',
      borderTopRightRadius: '16px',
      borderWidth: '1px',
      borderColor: 'border.default',
      boxShadow: 'large',
      zIndex: 'modal',
    })}
    transition:slide={{ duration: 250, axis: 'y' }}
  >
    <div
      class={flex({
        flexDirection: 'column',
        gap: '24px',
        padding: '20px',
        paddingBottom: '16px',
        overflowY: 'auto',
        flexGrow: '1',
      })}
    >
      {#each widgetsByCategory as { category, widgets } (category.id)}
        <div class={flex({ flexDirection: 'column', gap: '12px' })}>
          <div class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.default' })}>
            {category.name}
          </div>
          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            {#each widgets as widget (widget.type)}
              {@const WidgetComponent = WIDGET_COMPONENTS[widget.type]}
              {@const isAdded = addedWidgets.includes(widget.type)}
              <div
                class={cx(
                  'group',
                  css({
                    position: 'relative',
                    width: '300px',
                    opacity: isAdded ? '50' : '100',
                    cursor: isAdded ? 'not-allowed!' : 'grab!',
                  }),
                  !isAdded &&
                    css({
                      userSelect: 'none',
                    }),
                )}
                use:dragPaletteWidget={{
                  widgetType: widget.type,
                  isAdded,
                  onDragStart: (e) => {
                    onDragStart?.(e, widget.type);
                  },
                  onDragMove,
                  onDragEnd,
                  onDragCancel,
                }}
                use:tooltip={{
                  message: isAdded ? '이미 추가된 위젯이에요' : undefined,
                  placement: 'top',
                }}
              >
                {#if !isAdded}
                  <button
                    class={center({
                      position: 'absolute',
                      top: '0',
                      left: '0',
                      size: '28px',
                      borderRadius: 'full',
                      backgroundColor: 'surface.default',
                      borderWidth: '1px',
                      borderColor: 'border.default',
                      color: 'text.subtle',
                      opacity: '0',
                      transitionProperty: '[opacity]',
                      transitionDuration: '200ms',
                      transform: 'translate(-8px, -8px)',
                      _groupHover: { opacity: '100' },
                      _hover: { backgroundColor: 'surface.subtle', color: 'text.default' },
                      zIndex: '10',
                      cursor: 'pointer',
                    })}
                    data-widget-palette-button
                    onclick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      handleAddWidget(widget.type);
                    }}
                    onpointerdown={(e) => {
                      e.stopPropagation();
                    }}
                    type="button"
                  >
                    <Icon icon={PlusIcon} size={16} />
                  </button>
                {/if}
                <div class={css({ '& *': { pointerEvents: 'none!' } })}>
                  <WidgetComponent widgetId={`palette-preview-${widget.type}`} />
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>

    <div
      class={flex({
        justifyContent: 'space-between',
        alignItems: 'center',
        paddingX: '20px',
        paddingY: '8px',
        borderTop: '1px solid',
        borderColor: 'interactive.hover',
        backgroundColor: 'surface.muted',
      })}
    >
      <div class={css({ fontSize: '14px', color: 'text.faint' })}>위젯을 드래그해서 배치해 보세요</div>
      <Button
        onclick={() => {
          open = false;
        }}
        size="sm"
        variant="primary"
      >
        완료
      </Button>
    </div>
  </div>
{/if}
