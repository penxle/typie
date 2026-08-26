import { on } from 'svelte/events';
import { registerTooltipTrigger } from './tooltip-coordinator.svelte';
import type { Placement } from '@floating-ui/dom';
import type { Component, Snippet } from 'svelte';
import type { Action } from 'svelte/action';

type ModifierKey = 'Mod' | 'Ctrl' | 'Alt' | 'Shift';

export type TooltipParameter = {
  message?: string | Snippet | null;
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

export const tooltip: Action<HTMLElement, Parameter> = (element, parameter) => {
  let current = parameter;
  const description = ({
    message,
    trailing,
    trailingIcon,
    placement = 'bottom',
    offset = 8,
    delay = 500,
    force,
    arrow = true,
    keys,
  }: Parameter) => {
    const presentation =
      typeof message === 'function'
        ? { kind: 'wrapper' as const, message }
        : { kind: 'action' as const, message, trailing, trailingIcon, keys };

    return {
      element,
      container: element.ownerDocument.querySelector('.tooltip-container') ?? element.ownerDocument.body,
      eligible: Boolean(message),
      pinned: force === true,
      suppressed: force === false,
      delay,
      placement,
      offset,
      arrow,
      presentation,
    };
  };

  const registration = registerTooltipTrigger(description(current));
  const pointerenter = on(element, 'pointerenter', () => {
    registration.update(description(current));
    registration.enter();
  });
  const pointerleave = on(element, 'pointerleave', registration.leave);
  const click = on(
    element,
    'click',
    () => {
      if (!current.keepOnClick) registration.close();
    },
    { capture: true },
  );

  return {
    update: (next: Parameter) => {
      current = next;
      registration.update(description(current));
    },
    destroy: () => {
      pointerenter();
      pointerleave();
      click();
      registration.destroy();
    },
  };
};
