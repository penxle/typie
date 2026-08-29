import '../../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { parseMarkdown } from '$lib/markdown/parse';
import ChangelogMarkdown from './ChangelogMarkdown.svelte';

let component: Record<string, unknown> | null = null;

const render = (source: string) => {
  const target = document.createElement('div');
  target.style.fontSize = '15px';
  document.body.append(target);
  component = mount(ChangelogMarkdown, { target, props: { blocks: parseMarkdown(source) } });
  return target;
};

const settle = async (element: Element) => {
  await Promise.all(element.getAnimations().map((animation) => animation.finished));
};

afterEach(() => {
  if (component) {
    unmount(component);
    component = null;
  }
  document.body.replaceChildren();
});

describe('ChangelogMarkdown directives', () => {
  it('keeps the body collapsed and inert until the summary is clicked', async () => {
    const target = render('::: details 버그 수정\n- 첫번째 수정\n:::');

    const toggle = target.querySelector('button') as HTMLButtonElement;
    const reveal = toggle.nextElementSibling as HTMLElement;

    expect(toggle.getAttribute('aria-expanded')).toBe('false');
    expect(reveal.inert).toBe(true);
    expect(reveal.getBoundingClientRect().height).toBe(0);

    toggle.click();
    await tick();
    await settle(reveal);

    expect(toggle.getAttribute('aria-expanded')).toBe('true');
    expect(reveal.inert).toBe(false);
    expect(reveal.getBoundingClientRect().height).toBeGreaterThan(0);
  });

  it('animates the height and fades the body in', async () => {
    const target = render('::: details 버그 수정\n내용\n:::');

    const toggle = target.querySelector('button') as HTMLButtonElement;
    const reveal = toggle.nextElementSibling as HTMLElement;
    const body = reveal.querySelector(':scope > div > div') as HTMLElement;

    expect(getComputedStyle(reveal).transitionProperty).toBe('grid-template-rows');
    expect(getComputedStyle(body).transitionProperty).toBe('opacity');
    expect(getComputedStyle(body).opacity).toBe('0');
    expect(getComputedStyle(body).transitionDelay).toBe('0s');

    toggle.click();
    await tick();

    expect(reveal.getAnimations().length).toBeGreaterThan(0);

    expect(getComputedStyle(body).transitionDelay).toBe('0.1s');

    await settle(reveal);
    await settle(body);

    expect(getComputedStyle(body).opacity).toBe('1');
  });

  it('rotates the chevron only while the body is open', async () => {
    const target = render('::: details 버그 수정\n내용\n:::');

    const toggle = target.querySelector('button') as HTMLButtonElement;
    const chevron = target.querySelector(':scope button > svg') as Element;
    const upright = ['none', 'matrix(1, 0, 0, 1, 0, 0)'];

    await settle(chevron);
    expect(upright).toContain(getComputedStyle(chevron).transform);

    toggle.click();
    await tick();
    await settle(chevron);
    expect(getComputedStyle(chevron).transform).toBe('matrix(0, 1, -1, 0, 0, 0)');

    toggle.click();
    await tick();
    await settle(chevron);
    expect(upright).toContain(getComputedStyle(chevron).transform);
  });

  it('renders an empty summary without falling back to a browser label', () => {
    const target = render('::: details\n내용\n:::');

    expect(target.querySelector('button')?.textContent?.trim()).toBe('');
  });

  it('tracks each details block independently', async () => {
    const target = render('::: details 첫째\n가\n:::\n\n::: details 둘째\n나\n:::');

    const toggles = [...target.querySelectorAll('button')];
    expect(toggles).toHaveLength(2);

    toggles[1].click();
    await tick();

    expect(toggles[0].getAttribute('aria-expanded')).toBe('false');
    expect(toggles[1].getAttribute('aria-expanded')).toBe('true');
  });

  it('adds one body line of space per space directive', () => {
    const gapOf = (source: string) => {
      const target = render(source);
      const paragraphs = target.querySelectorAll('p');
      return paragraphs[1].getBoundingClientRect().top - paragraphs[0].getBoundingClientRect().bottom;
    };

    const plain = gapOf('가\n\n나');
    const spaced = gapOf('가\n\n::: space\n\n나');
    const doubled = gapOf('가\n\n::: space 2\n\n나');

    expect(spaced - plain).toBeCloseTo(26.25, 1);
    expect(doubled - plain).toBeCloseTo(52.5, 1);
  });

  it('renders note content smaller and dimmer than the surrounding body', () => {
    const target = render('본문\n\n::: note\n주석\n:::');

    const paragraphs = target.querySelectorAll('p');
    const body = getComputedStyle(paragraphs[0]);
    const note = getComputedStyle(paragraphs[1]);

    expect(Number.parseFloat(note.fontSize)).toBeLessThan(Number.parseFloat(body.fontSize));
    expect(note.color).not.toBe(body.color);
  });

  it('dims and shrinks every block inside a note, including headings and code', () => {
    const plain = render('# 제목\n\n```\n코드\n```');
    const plainHeading = getComputedStyle(plain.querySelector('[role="heading"]') as Element);
    const plainCode = getComputedStyle(plain.querySelector('pre') as Element);

    const noted = render('::: note\n# 제목\n\n```\n코드\n```\n:::');
    const notedHeading = getComputedStyle(noted.querySelector('[role="heading"]') as Element);
    const notedCode = getComputedStyle(noted.querySelector('pre') as Element);

    expect(Number.parseFloat(notedHeading.fontSize)).toBeLessThan(Number.parseFloat(plainHeading.fontSize));
    expect(Number.parseFloat(notedCode.fontSize)).toBeLessThan(Number.parseFloat(plainCode.fontSize));
    expect(notedHeading.color).not.toBe(plainHeading.color);
  });

  it('keeps the body type scale unchanged outside a note', () => {
    const target = render('# 하나\n\n## 둘\n\n본문\n\n```\n코드\n```');
    const headings = target.querySelectorAll('[role="heading"]');

    expect(getComputedStyle(headings[0]).fontSize).toBe('17px');
    expect(getComputedStyle(headings[1]).fontSize).toBe('16px');
    expect(getComputedStyle(target.querySelector('p') as Element).fontSize).toBe('15px');
    expect(getComputedStyle(target.querySelector('pre') as Element).fontSize).toBe('14px');
  });
});
