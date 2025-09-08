<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import { clamp, Ref } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { base64 } from 'rfc4648';
  import { tick, untrack } from 'svelte';
  import { fly } from 'svelte/transition';
  import * as Y from 'yjs';
  import { PostLayoutMode } from '@/enums';
  import IconClockFading from '~icons/lucide/clock-fading';
  import { fragment, graphql } from '$graphql';
  import type { Editor } from '@tiptap/core';
  import type { PageLayout } from '@typie/ui/utils';
  import type { PointerEventHandler } from 'svelte/elements';
  import type { Editor_Timeline_post } from '$graphql';

  type Props = {
    $post: Editor_Timeline_post;
    doc: Y.Doc;
    editor?: Ref<Editor>;
    viewDoc?: Y.Doc;
  };

  let { $post: _post, doc, editor, viewDoc = $bindable() }: Props = $props();

  let internalViewDoc = $state<Y.Doc>();

  const post = fragment(
    _post,
    graphql(`
      fragment Editor_Timeline_post on Post {
        id

        entity {
          id
          slug
        }
      }
    `),
  );

  const query = graphql(`
    query Editor_Timeline_Query($slug: String!) @client {
      post(slug: $slug) {
        id
        update

        snapshots {
          id
          snapshot
          createdAt
        }
      }
    }
  `);

  const { anchor, floating, arrow } = createFloatingActions({
    placement: 'top',
    offset: 8,
    arrow: true,
  });

  let value = $state(0);
  let showTooltip = $state(false);

  const baseDoc = new Y.Doc({ gc: false });

  const max = $derived($query ? $query.post.snapshots.length - 1 : 0);
  const p = $derived(max > 0 ? `${(value / max) * 100}%` : '0%');

  let initialized = $state(false);

  const initialize = async () => {
    const resp = await query.load({ slug: $post.entity.slug });

    Y.applyUpdateV2(baseDoc, base64.parse(resp.post.update));
    value = resp.post.snapshots.length - 1;

    initialized = true;
  };

  $effect(() => {
    internalViewDoc = new Y.Doc({ gc: false });

    untrack(() => initialize());

    return () => {
      internalViewDoc?.destroy();
      internalViewDoc = undefined;
      viewDoc?.destroy();
      viewDoc = undefined;
    };
  });

  $effect(() => {
    if (!$query || !initialized || !internalViewDoc) {
      return;
    }

    const snapshot = Y.decodeSnapshotV2(base64.parse($query.post.snapshots[value].snapshot));
    const snapshotDoc = Y.createDocFromSnapshot(baseDoc, snapshot);

    const currentStateVector = Y.encodeStateVector(internalViewDoc);
    const snapshotStateVector = Y.encodeStateVector(snapshotDoc);

    const missingUpdate = Y.encodeStateAsUpdateV2(internalViewDoc, snapshotStateVector);

    const undoManager = new Y.UndoManager(snapshotDoc, { trackedOrigins: new Set(['snapshot']) });
    Y.applyUpdateV2(snapshotDoc, missingUpdate, 'snapshot');
    undoManager.undo();

    const revertUpdate = Y.encodeStateAsUpdateV2(snapshotDoc, currentStateVector);
    Y.applyUpdateV2(internalViewDoc, revertUpdate, 'snapshot');

    viewDoc = internalViewDoc;

    untrack(() => {
      // viewEditor가 렌더링될 때까지 대기
      tick().then(() => {
        if (editor?.current) {
          const attrs = snapshotDoc.getMap('attrs');
          const layoutMode = (attrs.get('layoutMode') as PostLayoutMode) ?? PostLayoutMode.SCROLL;
          const pageLayout = (attrs.get('pageLayout') as PageLayout) ?? null;
          if (layoutMode === PostLayoutMode.PAGE && pageLayout) {
            editor.current.commands.setPageLayout(pageLayout);
          } else {
            editor.current.commands.clearPageLayout();
          }
        }
      });
    });
  });

  const restore = () => {
    if (!$query) {
      return;
    }

    const snapshot = Y.decodeSnapshotV2(base64.parse($query.post.snapshots[value].snapshot));
    const snapshotDoc = Y.createDocFromSnapshot(baseDoc, snapshot);

    const currentStateVector = Y.encodeStateVector(doc);
    const snapshotStateVector = Y.encodeStateVector(snapshotDoc);

    const missingUpdate = Y.encodeStateAsUpdateV2(doc, snapshotStateVector);

    const undoManager = new Y.UndoManager(snapshotDoc, { trackedOrigins: new Set(['snapshot']) });
    Y.applyUpdateV2(snapshotDoc, missingUpdate, 'snapshot');
    undoManager.undo();

    const revertUpdate = Y.encodeStateAsUpdateV2(snapshotDoc, currentStateVector);
    Y.applyUpdateV2(doc, revertUpdate, 'snapshot');
  };

  const handler: PointerEventHandler<HTMLElement> = (e) => {
    if (!e.currentTarget.parentElement || !$query) {
      return;
    }

    const { left: parentLeft, width: parentWidth } = e.currentTarget.parentElement.getBoundingClientRect();
    const { clientX: pointerLeft } = e;
    const ratio = clamp((pointerLeft - parentLeft) / parentWidth, 0, 1);
    value = Math.round(ratio * max);
  };
</script>

<div
  class={flex({
    position: 'absolute',
    left: '1/2',
    bottom: '32px',
    align: 'center',
    gap: '16px',
    borderRadius: '12px',
    padding: '12px',
    paddingRight: '16px',
    backgroundColor: 'white',
    border: '1px solid',
    borderColor: 'gray.200',
    translate: 'auto',
    translateX: '-1/2',
    boxShadow: '[0_8px_32px_rgba(0,0,0,0.08)]',
    zIndex: 'overEditor',
  })}
  in:fly={{ y: 32, duration: 250 }}
>
  <Icon style={css.raw({ color: 'gray.500' })} icon={IconClockFading} size={18} />

  {#if $query}
    <div class={flex({ position: 'relative', align: 'center', width: '420px', height: '36px', paddingX: '4px' })}>
      <div
        class={css({
          position: 'relative',
          borderRadius: 'full',
          width: 'full',
          height: '4px',
          backgroundColor: 'gray.200',
          overflow: 'hidden',
          cursor: 'pointer',
          transition: 'all',
          transitionDuration: '150ms',
          _hover: {
            backgroundColor: 'gray.300',
          },
        })}
        onpointerdown={handler}
      >
        <div
          style:width={p}
          class={css({
            height: 'full',
            backgroundColor: 'gray.900',
            transition: 'all',
            transitionDuration: '150ms',
            transitionTimingFunction: 'cubic-bezier(0.4, 0, 0.2, 1)',
          })}
        ></div>
      </div>
      <div class={css({ position: 'absolute', width: 'full', height: '36px', pointerEvents: 'none' })}>
        <button
          style:left={p}
          class={css({
            position: 'absolute',
            top: '1/2',
            borderRadius: 'full',
            size: '16px',
            backgroundColor: 'white',
            border: '2px solid',
            borderColor: 'gray.300',
            translate: 'auto',
            translateX: '-1/2',
            translateY: '-1/2',
            pointerEvents: 'auto',
            touchAction: 'none',
            cursor: 'ew-resize',
            transition: 'all',
            transitionDuration: '150ms',
            boxShadow: '[0_2px_8px_rgba(0,0,0,0.1)]',
            _hover: {
              scale: '[1.2]',
              boxShadow: '[0_4px_12px_rgba(0,0,0,0.15)]',
            },
            _active: {
              scale: '[1.1]',
            },
          })}
          aria-label="Timeline slider"
          ondragstart={(e: DragEvent) => e.preventDefault()}
          onpointerdown={(e: PointerEvent) => {
            showTooltip = true;
            (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
          }}
          onpointermove={(e: PointerEvent) => {
            e.preventDefault();
            if ((e.currentTarget as HTMLElement).hasPointerCapture(e.pointerId)) {
              handler(e as PointerEvent & { currentTarget: HTMLElement });
            }
          }}
          onpointerup={() => {
            showTooltip = false;
          }}
          type="button"
          use:anchor
        ></button>
      </div>
    </div>

    <div
      class={css({
        fontSize: '13px',
        fontFeatureSettings: '"tnum" 1',
        color: 'gray.700',
        whiteSpace: 'nowrap',
        minWidth: '150px',
        textAlign: 'center',
      })}
    >
      {dayjs($query.post.snapshots[value]?.createdAt).format('YYYY. MM. DD HH:mm')}
    </div>

    <div
      class={css({
        paddingX: '12px',
        paddingY: '8px',
        backgroundColor: 'gray.900',
        borderRadius: '8px',
        zIndex: '40',
        fontSize: '12px',
        color: 'white',
        whiteSpace: 'nowrap',
        pointerEvents: 'none',
        opacity: showTooltip ? '1' : '0',
        transition: 'opacity',
        transitionDuration: '150ms',
        boxShadow: '[0_4px_12px_rgba(0,0,0,0.15)]',
      })}
      role="tooltip"
      use:floating
    >
      {dayjs($query.post.snapshots[value]?.createdAt).format('YYYY. MM. DD HH:mm:ss')}
      <div
        class={css({
          size: '6px',
          backgroundColor: 'gray.900',
          zIndex: '1',
        })}
        use:arrow
      ></div>
    </div>

    <button
      class={css({
        display: 'flex',
        flexShrink: '0',
        alignItems: 'center',
        gap: '6px',
        paddingX: '14px',
        paddingY: '8px',
        backgroundColor: 'gray.900',
        color: 'white',
        borderRadius: '8px',
        fontSize: '13px',
        fontWeight: 'medium',
        cursor: 'pointer',
        transition: 'all',
        transitionDuration: '150ms',
        _hover: {
          backgroundColor: 'gray.800',
          transform: 'translateY(-1px)',
        },
        _active: {
          backgroundColor: 'black',
          transform: 'translateY(0)',
        },
      })}
      onclick={restore}
      type="button"
    >
      <Icon style={css.raw({ size: '14px' })} icon={IconClockFading} />
      복원
    </button>
  {:else}
    <div class={flex({ justify: 'center', align: 'center', width: '420px', height: '36px' })}>
      <RingSpinner style={css.raw({ size: '18px', color: 'gray.500' })} />
    </div>
  {/if}
</div>
