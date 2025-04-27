import { mount, unmount } from 'svelte';
import { on } from 'svelte/events';
import { createFloatingActions } from './floating.svelte';
import Tooltip from './TooltipComponent.svelte';
import type { Placement } from '@floating-ui/dom';
import type { Action } from 'svelte/action';

type Parameter = {
  message: string;
  placement?: Placement;
  offset?: number;
  delay?: number;
};

export const tooltip: Action<HTMLElement, Parameter> = (element, { message, placement = 'bottom', offset = 8, delay = 500 }: Parameter) => {
  let show = $state(false);
  let timer = $state<NodeJS.Timeout>();

  const { anchor, floating, arrow } = createFloatingActions({
    placement,
    offset,
    arrow: true,
  });

  const props = $state({
    message,
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

    const click = on(element, 'click', () => {
      if (timer) {
        clearTimeout(timer);
      }

      show = false;
    });

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
    update: ({ message }: Parameter) => {
      props.message = message;
    },
  };
};
