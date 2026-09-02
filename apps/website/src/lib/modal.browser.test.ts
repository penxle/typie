import '../app.css';

import { Modal } from '@typie/ui/components';
import { createRawSnippet, mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { userEvent } from 'vitest/browser';

let component: Record<string, unknown> | undefined;

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

const settle = async () => {
  await tick();
  await frame();
  await frame();
  await new Promise((resolve) => setTimeout(resolve, 20));
};

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

const clickOutside = async () => {
  const outside = document.createElement('button');
  outside.textContent = 'outside';
  document.body.append(outside);
  await userEvent.click(outside);
  await settle();
  expect(document.activeElement).toBe(outside);
};

const open = async (props: Record<string, unknown> = {}) => {
  component = mount(Modal, {
    target: document.body,
    props: {
      open: true,
      closable: false,
      children: createRawSnippet(() => ({
        render: () => `<div><button id="first" type="button">first</button><p id="text">text</p><input id="field" /></div>`,
      })),
      ...props,
    },
  });
  await settle();

  const dialog = document.querySelector('[role="dialog"]');
  const first = document.querySelector('#first');
  const text = document.querySelector('#text');
  const field = document.querySelector('#field');
  if (
    !(dialog instanceof HTMLElement) ||
    !(first instanceof HTMLElement) ||
    !(text instanceof HTMLElement) ||
    !(field instanceof HTMLElement)
  ) {
    throw new TypeError('modal content missing');
  }

  return { dialog, first, text, field };
};

describe('shared modal focus', () => {
  it('puts initial focus on the dialog container and never paints a ring on it', async () => {
    await clickOutside();
    const { dialog, first } = await open();

    expect(document.activeElement).toBe(dialog);
    expect(first.matches(':focus-visible')).toBe(false);

    await userEvent.keyboard('z');
    await settle();
    expect(document.activeElement).toBe(dialog);
    expect(dialog.matches(':focus-visible')).toBe(true);
    expect(getComputedStyle(dialog).outlineWidth).toBe('0px');

    await userEvent.keyboard('{Tab}');
    await settle();
    expect(document.activeElement).toBe(first);
  });

  it('keeps the dialog ring off after a click inside the content', async () => {
    const { dialog, text } = await open();

    await userEvent.click(text);
    await settle();
    expect(document.activeElement).toBe(dialog);

    await userEvent.keyboard('z');
    await settle();
    expect(getComputedStyle(dialog).outlineWidth).toBe('0px');
  });

  it('keeps focus on a child that focused itself before the trap settled', async () => {
    component = mount(Modal, {
      target: document.body,
      props: {
        open: true,
        closable: false,
        children: createRawSnippet(() => ({
          render: () => `<div><button id="first" type="button">first</button><input id="field" /></div>`,
        })),
      },
    });
    await tick();
    const field = document.querySelector('#field');
    if (!(field instanceof HTMLElement)) throw new TypeError('field missing');
    field.focus();
    expect(document.activeElement).toBe(field);
    await settle();

    expect(document.activeElement).toBe(field);
  });

  it('honors an explicit initialFocus override', async () => {
    const { field } = await open({ focusTrapOptions: { initialFocus: '#field' } });

    expect(document.activeElement).toBe(field);
  });
});
