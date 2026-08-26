<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { on } from 'svelte/events';
  import { registerTooltipTrigger } from '../actions/tooltip-coordinator.svelte';
  import type { Placement } from '@floating-ui/dom';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { Action } from 'svelte/action';

  type Props = {
    message?: string | Snippet;
    style?: SystemStyleObject;
    tooltipStyle?: SystemStyleObject;
    offset?: number;
    enabled?: boolean;
    placement?: Placement;
    keepShowing?: boolean;
    children: Snippet;
  };

  let { message, style, tooltipStyle, offset, enabled = true, placement = 'bottom', keepShowing = false, children }: Props = $props();

  type Parameter = Pick<Props, 'message' | 'tooltipStyle' | 'offset' | 'enabled' | 'placement' | 'keepShowing'>;

  const tooltipTrigger: Action<HTMLElement, Parameter> = (element, parameter) => {
    let current = parameter;
    const description = () => ({
      element,
      container: element.closest('dialog, [popover]') ?? element.ownerDocument.body,
      eligible: current.enabled ?? true,
      pinned: current.keepShowing ?? false,
      suppressed: false,
      delay: 0,
      placement: current.placement ?? 'bottom',
      offset: current.offset ?? 6,
      arrow: true,
      presentation: { kind: 'wrapper' as const, message: current.message, tooltipStyle: current.tooltipStyle },
    });
    const registration = registerTooltipTrigger(description());
    const pointerenter = on(element, 'pointerenter', registration.enter);
    const pointerleave = on(element, 'pointerleave', registration.leave);

    return {
      update: (next) => {
        current = next;
        registration.update(description());
      },
      destroy: () => {
        pointerenter();
        pointerleave();
        registration.destroy();
      },
    };
  };
</script>

<div class={css(style)} use:tooltipTrigger={{ message, tooltipStyle, offset, enabled, placement, keepShowing }}>
  {@render children()}
</div>
