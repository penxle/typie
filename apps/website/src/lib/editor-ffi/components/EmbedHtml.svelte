<script lang="ts">
  import { connectIframely, isIframelyIframe } from '../iframely';
  import type { Snippet } from 'svelte';

  type Props = {
    html: string | null | undefined;
    layoutReady: boolean;
    scrollRoot: Element | null;
    class?: string;
    fallback: Snippet;
  };

  let { html, layoutReady, scrollRoot, class: className, fallback }: Props = $props();
  let failed = $state(false);

  function mountHtml(container: HTMLElement) {
    const content = document.createElement('div');
    content.dataset.embedHtml = '';
    content.innerHTML = html ?? '';
    const frames = [...content.querySelectorAll('iframe')];
    // Responsive HTML already carries the layout. Only defer browsing contexts
    // that cannot affect its size. Hosted widgets keep their own loading policy
    // and report any size corrections through the SDK.
    const defer = frames.length > 0 && !content.querySelector('script, style') && frames.every(canDeferIframe);
    const pending = defer
      ? frames.map((frame) => {
          const placeholder = document.createComment('iframe');
          frame.replaceWith(placeholder);
          return { frame, placeholder };
        })
      : [];
    // Keep the SDK's removable wrapper inside the DOM owned by this component.
    container.replaceChildren(content);
    const disconnect = connectIframely(frames, () => {
      failed = true;
    });

    $effect(() => {
      // Initial bounds may overlap until measured heights reach publication.
      // Read readiness here so geometry updates never remount the HTML.
      if (!layoutReady || pending.length === 0) return;
      const observer = new IntersectionObserver(
        (entries) => {
          if (!layoutReady || entries.every((entry) => !entry.isIntersecting)) return;
          observer.disconnect();
          for (const { frame, placeholder } of pending) placeholder.replaceWith(frame);
          pending.length = 0;
        },
        { root: scrollRoot, rootMargin: '200px 0px' },
      );
      observer.observe(content);
      return () => observer.disconnect();
    });

    return () => {
      disconnect?.();
      pending.length = 0;
      container.replaceChildren();
    };
  }

  function canDeferIframe(frame: HTMLIFrameElement): boolean {
    return frame.style.position === 'absolute' && frame.hasAttribute('src') && !isIframelyIframe(frame);
  }
</script>

{#if html && !failed}
  <div class={className} {@attach mountHtml}></div>
{:else}
  {@render fallback()}
{/if}
