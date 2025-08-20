<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Canvas, CanvasEditor } from '@typie/ui/canvas';
  import { Icon } from '@typie/ui/components';
  import { base64 } from 'rfc4648';
  import { onMount } from 'svelte';
  import * as Y from 'yjs';
  import EyeIcon from '~icons/lucide/eye';
  import { graphql } from '$graphql';
  import Zoom from './Zoom.svelte';

  const query = graphql(`
    query WebViewCanvasPage_Query($slug: String!, $siteId: ID!) {
      entity(slug: $slug) {
        id
        slug

        node {
          __typename

          ... on Canvas {
            id
            title
            update
          }
        }
      }

      site(siteId: $siteId) {
        id
      }
    }
  `);

  const viewEntity = graphql(`
    mutation WebViewCanvas_ViewEntity_Mutation($input: ViewEntityInput!) {
      viewEntity(input: $input) {
        id
      }
    }
  `);

  let canvas = $state<Canvas>();
  let doc = new Y.Doc();

  $effect(() => {
    if ($query.entity?.node.__typename === 'Canvas') {
      // Yjs 문서에 업데이트 적용
      const update = $query.entity.node.update;

      if (update) {
        Y.applyUpdateV2(doc, base64.parse(update), 'remote');
      }
    }
  });

  onMount(() => {
    viewEntity({ entityId: $query.entity.id });

    if (canvas) {
      const { x, y, width, height } = canvas.scene.getLayer().getClientRect();
      const stageWidth = canvas.stage.width();
      const stageHeight = canvas.stage.height();

      canvas.moveTo(-(x + width / 2 - stageWidth / 2), -(y + height / 2 - stageHeight / 2));

      // NOTE: 여유도 주고 node 없을 때 div0 되지 않게 100 더함
      canvas.scaleTo(Math.min(stageWidth / (width + 100), stageHeight / (height + 100), 1));
    }

    window.__webview__?.emitEvent('webviewReady');
  });
</script>

<div
  class={css({
    position: 'relative',
    width: 'full',
    height: '[100dvh]',
    overflow: 'hidden',
    backgroundColor: 'surface.subtle',
    userSelect: 'none',
    touchAction: 'none',
    WebkitTouchCallout: 'none',
    WebkitOverflowScrolling: 'touch',
  })}
>
  <div
    class={css({
      size: 'full',
    })}
  >
    <CanvasEditor style={css.raw({ width: 'full', height: 'full' })} {doc} readonly bind:canvas />
  </div>

  {#if canvas}
    <Zoom {canvas} />
  {/if}

  <button
    class={css({
      position: 'absolute',
      left: '40px',
      bottom: '40px',
      display: 'flex',
      alignItems: 'center',
      gap: '5px',
      paddingX: '12px',
      paddingY: '8px',
      backgroundColor: 'surface.default',
      borderRadius: '6px',
      borderWidth: '1px',
      borderStyle: 'solid',
      borderColor: 'border.default',
    })}
    onclick={() => {
      window.__webview__?.emitEvent('readOnlyBadgeClick');
    }}
    type="button"
  >
    <Icon style={css.raw({ color: 'text.subtle' })} icon={EyeIcon} size={16} />
    <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.subtle' })}>보기 전용</span>
  </button>
</div>
