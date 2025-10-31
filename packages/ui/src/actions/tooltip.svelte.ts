import { mount, unmount } from 'svelte';
import { on } from 'svelte/events';
import { debounce } from '../utils';
import { createFloatingActions } from './floating.svelte';
import Tooltip from './TooltipComponent.svelte';
import type { Placement } from '@floating-ui/dom';
import type { Component } from 'svelte';
import type { Action } from 'svelte/action';

type ModifierKey = 'Mod' | 'Ctrl' | 'Alt' | 'Shift';

export type TooltipParameter = {
  message?: string | null;
  trailing?: string;
  trailingIcon?: Component;
  placement?: Placement;
  keys?: [...ModifierKey[], string];
  offset?: number;
  delay?: number;
  keepOnClick?: boolean;
  force?: boolean;
  arrow?: boolean;
};

type Parameter = TooltipParameter;

export const tooltip: Action<HTMLElement, Parameter> = (
  element,
  {
    message,
    trailing,
    trailingIcon,
    placement = 'bottom',
    offset = 8,
    delay = 500,
    keepOnClick = false,
    force,
    arrow = true,
    keys,
  }: Parameter,
) => {
  let show = $state(false);
  let forceShow = $state(force);

  let shouldShow = $state(false);

  const debouncedShouldShow = debounce(() => {
    shouldShow = forceShow ?? show;
  }, 0);

  $effect(() => {
    void show;
    void forceShow;

    debouncedShouldShow();
  });

  let timer = $state<NodeJS.Timeout>();

  const {
    anchor,
    floating,
    arrow: arrowAction,
  } = createFloatingActions({
    placement,
    offset,
    arrow,
  });

  const props = $state({
    message,
    trailing,
    trailingIcon,
    keys,
    floating,
    arrow: arrow ? arrowAction : undefined,
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
    update: ({ message, trailing, trailingIcon, keys, force }: Parameter) => {
      props.message = message;
      props.trailing = trailing;
      props.trailingIcon = trailingIcon;
      props.keys = keys;
      forceShow = force;
    },
  };
};
