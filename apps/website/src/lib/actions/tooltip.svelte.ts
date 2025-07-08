import { mount, unmount } from 'svelte';
import { on } from 'svelte/events';
import { createFloatingActions } from './floating.svelte';
import Tooltip from './TooltipComponent.svelte';
import type { Placement } from '@floating-ui/dom';
import type { Action } from 'svelte/action';

type Parameter = {
  message: string;
  trailing?: string;
  placement?: Placement;
  offset?: number;
  delay?: number;
  keepOnClick?: boolean;
};

export const tooltip: Action<HTMLElement, Parameter> = (
  element,
  { message, trailing, placement = 'bottom', offset = 8, delay = 500, keepOnClick = false }: Parameter,
) => {
  let show = $state(false);
  let timer = $state<NodeJS.Timeout>();

  const { anchor, floating, arrow } = createFloatingActions({
    placement,
    offset,
    arrow: true,
  });

  const props = $state({
    message,
    trailing,
    floating,
    arrow,
  });

  $effect(() => {
    const mouseenter = on(element, 'mouseenter', () => {
      if (timer) {
        clearTimeout(timer);
      }

      timer = setTimeout(() => {
        show = true;
      }, delay);
    });

    const mouseleave = on(element, 'mouseleave', () => {
      if (timer) {
        clearTimeout(timer);
      }

      show = false;
    });

    const click = on(
      element,
      'click',
      () => {
        if (keepOnClick) {
          return;
        }

        if (timer) {
          clearTimeout(timer);
        }

        show = false;
      },
      { capture: true },
    );

    anchor(element);

    return () => {
      mouseenter();
      mouseleave();
      click();
    };
  });

  $effect(() => {
    if (show) {
      const component = mount(Tooltip, {
        target: document.body,
        props,
        intro: true,
      });

      return () => {
        unmount(component, { outro: true });
      };
    }
  });

  return {
    update: ({ message, trailing }: Parameter) => {
      props.message = message;
      props.trailing = trailing;
    },
  };
};
