import { mount, unmount } from 'svelte';
import { on } from 'svelte/events';
import { debounce } from '$lib/utils';
import { createFloatingActions } from './floating.svelte';
import Tooltip from './TooltipComponent.svelte';
import type { Placement } from '@floating-ui/dom';
import type { Action } from 'svelte/action';

type ModifierKey = 'Mod' | 'Ctrl' | 'Alt' | 'Shift';

type Parameter = {
  message: string;
  trailing?: string;
  placement?: Placement;
  keys?: [...ModifierKey[], string];
  offset?: number;
  delay?: number;
  keepOnClick?: boolean;
  force?: boolean;
};

export const tooltip: Action<HTMLElement, Parameter> = (
  element,
  { message, trailing, placement = 'bottom', offset = 8, delay = 500, keepOnClick = false, force = false, keys }: Parameter,
) => {
  let show = $state(false);
  let forceShow = $state(force);

  let shouldShow = $state(false);

  const debouncedShouldShow = debounce(() => {
    shouldShow = show || forceShow;
  }, 0);

  $effect(() => {
    void show;
    void forceShow;

    debouncedShouldShow();
  });

  let timer = $state<NodeJS.Timeout>();

  const { anchor, floating, arrow } = createFloatingActions({
    placement,
    offset,
    arrow: true,
  });

  const props = $state({
    message,
    trailing,
    keys,
    floating,
    arrow,
  });

  $effect(() => {
    const pointerenter = on(element, 'pointerenter', () => {
      if (timer) {
        clearTimeout(timer);
      }

      if (delay > 0) {
        timer = setTimeout(() => {
          show = true;
        }, delay);
      } else {
        show = true;
      }
    });

    const pointerleave = on(element, 'pointerleave', () => {
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
      pointerenter();
      pointerleave();
      click();
    };
  });

  $effect(() => {
    if (shouldShow) {
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
    update: ({ message, trailing, keys, force = false }: Parameter) => {
      props.message = message;
      props.trailing = trailing;
      props.keys = keys;
      forceShow = force;
    },
  };
};
