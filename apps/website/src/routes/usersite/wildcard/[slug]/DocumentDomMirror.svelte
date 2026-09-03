<script lang="ts">
  import * as Sentry from '@sentry/sveltekit';
  import { css } from '@typie/styled-system/css';
  import { createDocumentDomMirror, startDocumentDomProjection } from '$lib/editor-ffi/document-dom-mirror';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';

  type Props = {
    editor?: Editor;
    excerpt?: string | null;
  };

  let { editor, excerpt }: Props = $props();
  let host = $state<HTMLDivElement>();

  const reportError = (error: unknown, phase: 'initialization' | 'projection') => {
    console.error(error);
    try {
      Sentry.captureException(error, {
        tags: {
          feature: 'document-dom-mirror',
          phase,
        },
      });
    } catch (err) {
      console.error(err);
    }
  };

  $effect(() => {
    const element = host;
    if (!element || !editor) return;

    try {
      const mirror = createDocumentDomMirror(editor.documentDomProjection());
      element.replaceChildren(mirror.element);

      const stop = startDocumentDomProjection({
        mirror,
        apply: (doc) => editor.setDoc(doc),
        reportError: (error) => reportError(error, 'projection'),
      });

      return () => {
        stop();
        element.replaceChildren();
      };
    } catch (err) {
      reportError(err, 'initialization');
      element.replaceChildren();
    }
  });
</script>

<div
  bind:this={host}
  class={css({
    position: 'absolute',
    width: '1px',
    height: '1px',
    overflow: 'hidden',
    clipPath: 'inset(50%)',
    whiteSpace: 'nowrap',
    pointerEvents: 'none',
    userSelect: 'none',
  })}
  aria-hidden="true"
  inert
  translate="yes"
>
  {#if !editor && excerpt}
    <p data-typie-dom-mirror-seed>{excerpt}</p>
  {/if}
</div>
