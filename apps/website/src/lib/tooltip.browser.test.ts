import '../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import TooltipTestRoot from './tooltip-test-root.svelte';

let component: Record<string, unknown> | undefined;

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

const frames = async (count = 2) => {
  for (let index = 0; index < count; index++) await frame();
};

const trigger = (testId: string): HTMLElement => {
  const element = document.querySelector<HTMLElement>(`[data-testid="${testId}"]`);
  if (!element) throw new Error(`Missing trigger: ${testId}`);
  return element;
};

const wrapperAnchor = (testId: string): HTMLElement => {
  const anchor = trigger(testId).parentElement;
  if (!(anchor instanceof HTMLElement)) throw new Error(`Missing wrapper anchor: ${testId}`);
  return anchor;
};

const tooltip = (): HTMLElement | null => document.querySelector<HTMLElement>('[role="tooltip"]');

const enter = (element: HTMLElement) => element.dispatchEvent(new PointerEvent('pointerenter'));
const leave = (element: HTMLElement) => element.dispatchEvent(new PointerEvent('pointerleave'));
const noop = () => null;

const mountFixture = async ({ reducedMotion = false } = {}) => {
  vi.spyOn(window, 'matchMedia').mockImplementation(
    (query) =>
      ({
        matches: query === '(prefers-reduced-motion: reduce)' && reducedMotion,
        media: query,
        onchange: null,
        addEventListener: noop,
        removeEventListener: noop,
        addListener: noop,
        removeListener: noop,
        dispatchEvent: () => true,
      }) satisfies MediaQueryList,
  );
  component = mount(TooltipTestRoot, { target: document.body, props: { browserLayout: true } });
  await tick();
  await frames();
};

const waitForTooltip = async (text: string): Promise<HTMLElement> => {
  await vi.waitFor(() => {
    expect(document.querySelectorAll('[role="tooltip"]')).toHaveLength(1);
    expect(tooltip()?.textContent).toContain(text);
  });
  const element = tooltip();
  if (!element) throw new Error('Expected a tooltip');
  return element;
};

const waitForPositionedTooltip = async (text: string): Promise<HTMLElement> => {
  const element = await waitForTooltip(text);
  await vi.waitFor(() => expect(element.style.visibility).toBe('visible'));
  return element;
};

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  for (const element of document.querySelectorAll<HTMLElement>('[data-tooltip-presence]')) {
    for (const animation of element.getAnimations()) animation.finish();
  }
  await frames(1);
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

describe('shared tooltip anchor motion', () => {
  it('keeps the nearby shell visible while its content crossfades and the shell travels in both directions', async () => {
    await mountFixture();
    const first = trigger('motion-first');
    const second = trigger('motion-second');

    enter(first);
    const host = await waitForPositionedTooltip('굵게');
    const surface = host.querySelector<HTMLElement>('[data-tooltip-surface]');
    const firstLeft = Number.parseFloat(host.style.left);

    leave(first);
    enter(second);
    await frames(1);

    expect(tooltip()).toBe(host);
    expect(host.dataset.tooltipMotion).toBe('travel');
    const outgoingContent = host.querySelector<HTMLElement>('[data-tooltip-content="outgoing"]');
    const currentContent = host.querySelector<HTMLElement>('[data-tooltip-content="current"]');
    expect(outgoingContent?.textContent).toContain('굵게');
    expect(currentContent?.textContent).toContain('기울임');
    expect(getComputedStyle(host).opacity).toBe('1');
    expect(
      host
        .getAnimations()
        .some((animation) => (animation.effect as KeyframeEffect | null)?.getKeyframes().some((frame) => frame.opacity !== undefined)),
    ).toBe(false);
    expect(getComputedStyle(currentContent as HTMLElement).opacity).toBe('0');
    const outgoingAnimation = outgoingContent?.getAnimations()[0];
    const incomingAnimation = currentContent?.getAnimations()[0];
    const sizeAnimation = surface
      ?.getAnimations()
      .find((animation) => (animation.effect as KeyframeEffect | null)?.getKeyframes().some((frame) => frame.width !== undefined));
    expect(outgoingAnimation).toBeDefined();
    expect(incomingAnimation).toBeDefined();
    expect(sizeAnimation).toBeDefined();
    expect(getComputedStyle(surface as HTMLElement).overflow).toBe('hidden');
    expect(Math.abs(Number.parseFloat(host.style.left) - firstLeft)).toBeLessThanOrEqual(160);
    const midpoint = Number(sizeAnimation?.effect?.getTiming().duration ?? 0) / 2;
    for (const animation of [outgoingAnimation, incomingAnimation, sizeAnimation]) {
      if (!animation) continue;
      animation.pause();
      animation.currentTime = midpoint;
    }
    await frames(1);
    expect(currentContent?.getBoundingClientRect().top).toBeCloseTo(outgoingContent?.getBoundingClientRect().top ?? 0, 1);
    expect(currentContent?.getBoundingClientRect().height).toBeCloseTo(outgoingContent?.getBoundingClientRect().height ?? 0, 1);
    outgoingAnimation?.finish();
    incomingAnimation?.finish();
    sizeAnimation?.finish();
    await frames(1);
    expect(host.querySelector('[data-tooltip-content="outgoing"]')).toBeNull();
    expect(getComputedStyle(currentContent as HTMLElement).opacity).toBe('1');
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('idle'));

    leave(second);
    enter(first);
    await waitForTooltip('굵게');
    expect(tooltip()).toBe(host);
    expect(host.dataset.tooltipMotion).toBe('travel');
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('idle'));
  });

  it('preserves the full outgoing layout when a width transition reverses midway', async () => {
    await mountFixture();
    const lock = trigger('motion-toolbar-lock');
    const zen = trigger('motion-toolbar-zen');

    enter(lock);
    const host = await waitForPositionedTooltip('편집 잠금');
    host.querySelector<HTMLElement>('[data-tooltip-presence]')?.getAnimations()[0]?.finish();
    await frames(1);

    leave(lock);
    enter(zen);
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('travel'));
    const firstSurface = host.querySelector<HTMLElement>('[data-tooltip-surface]');
    const firstIncoming = host.querySelector<HTMLElement>('[data-tooltip-content="current"]');
    const sizeAnimation = firstSurface
      ?.getAnimations()
      .find((animation) => (animation.effect as KeyframeEffect | null)?.getKeyframes().some((frame) => frame.width !== undefined));
    const midpoint = Number(sizeAnimation?.effect?.getTiming().duration ?? 0) / 2;
    for (const animation of [
      ...host.getAnimations(),
      ...(firstSurface?.getAnimations() ?? []),
      ...(firstIncoming?.getAnimations() ?? []),
    ]) {
      animation.pause();
      animation.currentTime = midpoint;
    }
    const fullZenWidth = Number.parseFloat(firstIncoming?.style.width ?? '');
    expect(fullZenWidth).toBeGreaterThan(0);

    leave(zen);
    enter(lock);
    await vi.waitFor(() => {
      expect(host.querySelector<HTMLElement>('[data-tooltip-content="outgoing"]')?.textContent).toContain('집중 모드 켜기');
      expect(host.querySelector<HTMLElement>('[data-tooltip-content="current"]')?.textContent).toContain('편집 잠금');
    });

    const outgoing = host.querySelector<HTMLElement>('[data-tooltip-content="outgoing"]');
    const outgoingLabel = outgoing?.querySelector<HTMLElement>('span');
    expect(outgoingLabel).not.toBeNull();
    const fontSize = Number.parseFloat(getComputedStyle(outgoingLabel as HTMLElement).fontSize);
    expect(outgoingLabel?.getBoundingClientRect().height).toBeLessThan(fontSize * 1.5);
    expect(Number.parseFloat(outgoing?.style.width ?? '')).toBeCloseTo(fullZenWidth, 1);
  });

  it('preserves a stable wide tooltip layout during a single transition to a narrower target', async () => {
    await mountFixture();
    const zen = trigger('motion-toolbar-zen');
    const lock = trigger('motion-toolbar-lock');

    enter(zen);
    const host = await waitForPositionedTooltip('집중 모드 켜기');
    host.querySelector<HTMLElement>('[data-tooltip-presence]')?.getAnimations()[0]?.finish();
    await frames(1);
    const stableContent = host.querySelector<HTMLElement>('[data-tooltip-content="current"]');
    const stableIntrinsicWidth = stableContent?.scrollWidth ?? 0;
    expect(stableIntrinsicWidth).toBeGreaterThan(0);

    leave(zen);
    enter(lock);
    await vi.waitFor(() => {
      expect(host.querySelector<HTMLElement>('[data-tooltip-content="outgoing"]')?.textContent).toContain('집중 모드 켜기');
    });

    const outgoing = host.querySelector<HTMLElement>('[data-tooltip-content="outgoing"]');
    const outgoingLabel = outgoing?.querySelector<HTMLElement>('span');
    expect(outgoingLabel).not.toBeNull();
    const fontSize = Number.parseFloat(getComputedStyle(outgoingLabel as HTMLElement).fontSize);
    expect(outgoingLabel?.getBoundingClientRect().height).toBeLessThan(fontSize * 1.5);
    expect(Number.parseFloat(outgoing?.style.width ?? '')).toBeGreaterThanOrEqual(stableIntrinsicWidth);
  });

  it('pins the arrow to the next anchor immediately while the shared shell travels underneath it', async () => {
    await mountFixture();
    const close = trigger('motion-toolbar-close');
    const zen = trigger('motion-toolbar-zen');

    enter(close);
    const host = await waitForPositionedTooltip('창 닫기');
    host.querySelector<HTMLElement>('[data-tooltip-presence]')?.getAnimations()[0]?.finish();
    await frames(1);
    const arrow = host.querySelector<HTMLElement>('[data-tooltip-surface]')?.nextElementSibling as HTMLElement | null;
    expect(arrow).not.toBeNull();
    const nextAnchorCenter = zen.getBoundingClientRect().left + zen.getBoundingClientRect().width / 2;

    leave(close);
    enter(zen);
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('travel'));
    const travel = host
      .getAnimations()
      .find((animation) => (animation.effect as KeyframeEffect | null)?.getKeyframes().some((keyframe) => keyframe.left !== undefined));
    expect(travel).toBeDefined();
    const arrowMotion = arrow?.getAnimations()[0];
    expect(arrowMotion).toBeDefined();
    const duration = Number(travel?.effect?.getTiming().duration ?? 0);
    for (const progress of [0, 0.5, 0.99]) {
      for (const animation of [travel, arrowMotion]) {
        animation?.pause();
        if (animation) animation.currentTime = duration * progress;
      }
      await frames(1);
      const arrowCenter = (arrow?.getBoundingClientRect().left ?? 0) + (arrow?.getBoundingClientRect().width ?? 0) / 2;
      expect(arrowCenter).toBeCloseTo(nextAnchorCenter, 0);
    }
  });

  it('crossfades without travelling to a far anchor', async () => {
    await mountFixture();
    const first = trigger('motion-first');
    const far = trigger('motion-far');

    enter(first);
    const host = await waitForPositionedTooltip('굵게');
    const firstLeft = Number.parseFloat(host.style.left);

    leave(first);
    enter(far);
    await frames(1);

    expect(tooltip()).toBe(host);
    expect(host.dataset.tooltipMotion).toBe('crossfade');
    expect(host.textContent).toContain('굵게');
    await waitForTooltip('Motion far');
    expect(Math.abs(Number.parseFloat(host.style.left) - firstLeft)).toBeGreaterThan(160);
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('idle'));
  });

  it('classifies the transition from the new content final placement', async () => {
    await mountFixture();
    const first = trigger('motion-bottom-first');
    const tall = wrapperAnchor('motion-bottom-tall');

    expect(tall.getBoundingClientRect().top).toBeCloseTo(first.getBoundingClientRect().top, 0);
    expect(tall.getBoundingClientRect().left).toBeCloseTo(first.getBoundingClientRect().left, 0);

    enter(first);
    const host = await waitForPositionedTooltip('Bottom short');

    leave(first);
    enter(tall);
    await frames(2);

    expect(host.dataset.tooltipMotion).toBe('crossfade');
    expect(host.textContent).toContain('Bottom short');
    await waitForTooltip('Tall line one');
  });

  it('retargets nearby travel from the current visual position', async () => {
    await mountFixture();
    const first = trigger('motion-first');
    const second = trigger('motion-second');
    const third = trigger('motion-third');

    enter(first);
    const host = await waitForPositionedTooltip('굵게');
    leave(first);
    enter(second);
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('travel'));

    const firstTravel = host
      .getAnimations()
      .find((animation) => (animation.effect as KeyframeEffect | null)?.getKeyframes().some((keyframe) => keyframe.left !== undefined));
    expect(firstTravel).toBeDefined();
    if (!firstTravel) throw new Error('Expected the first travel animation');
    firstTravel.pause();
    firstTravel.currentTime = 60;
    await frames(1);
    const currentLeft = Number.parseFloat(getComputedStyle(host).left);

    leave(second);
    enter(third);
    await frames(1);
    const nextTravel = host
      .getAnimations()
      .find((animation) => (animation.effect as KeyframeEffect | null)?.getKeyframes()[0]?.left !== undefined);
    expect(nextTravel).toBeDefined();
    const startingLeft = Number.parseFloat(String((nextTravel?.effect as KeyframeEffect | null)?.getKeyframes()[0]?.left));
    expect(startingLeft).toBeCloseTo(currentLeft, 0);
  });

  it('keeps each wrapper content layer at its measured size while crossfading', async () => {
    await mountFixture();
    const first = wrapperAnchor('motion-wrapper-first');
    const second = wrapperAnchor('motion-wrapper-second');

    enter(first);
    const host = await waitForPositionedTooltip('wraps onto');
    host.querySelector<HTMLElement>('[data-tooltip-presence]')?.getAnimations()[0]?.finish();
    await frames(1);
    const previousContent = host.querySelector<HTMLElement>('[data-tooltip-content="current"]')?.getBoundingClientRect();
    expect(previousContent).toBeDefined();

    leave(first);
    enter(second);
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('travel'));
    const outgoing = host.querySelector<HTMLElement>('[data-tooltip-content="outgoing"]');
    const outgoingRect = outgoing?.getBoundingClientRect();

    expect(outgoingRect?.width).toBeCloseTo(previousContent?.width ?? 0, 0);
    expect(outgoingRect?.height).toBeCloseTo(previousContent?.height ?? 0, 0);
  });

  it('keeps the incoming action label on one line while its content crossfades', async () => {
    await mountFixture();
    const list = trigger('motion-prism-list');
    const close = trigger('motion-prism-close');
    Object.assign(list.style, { position: 'fixed', top: '80px', right: '48px', left: 'auto' });
    Object.assign(close.style, { position: 'fixed', top: '80px', right: '8px', left: 'auto' });

    enter(list);
    const host = await waitForPositionedTooltip('대화 목록 열기');
    host.style.left = `${window.innerWidth - 92}px`;
    leave(list);
    enter(close);
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('travel'));

    const incoming = host.querySelector<HTMLElement>('[data-tooltip-content="current"]');
    const label = incoming?.querySelector<HTMLElement>('span');
    expect(label).not.toBeNull();
    const fontSize = Number.parseFloat(getComputedStyle(label as HTMLElement).fontSize);
    expect(label?.getBoundingClientRect().height).toBeLessThan(fontSize * 1.5);
  });

  it('does not overlap first-show animation with an immediate shared transition', async () => {
    await mountFixture();
    const first = trigger('motion-first');
    const second = trigger('motion-second');

    enter(first);
    const host = await waitForTooltip('굵게');
    await frames(2);
    leave(first);
    enter(second);
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('travel'));

    const rootOpacityAnimations = host
      .getAnimations()
      .filter((animation) =>
        (animation.effect as KeyframeEffect | null)?.getKeyframes().some((keyframe) => keyframe.opacity !== undefined),
      );
    expect(rootOpacityAnimations).toHaveLength(0);
  });

  it('returns to fully visible without a snap when an outro interrupts the intro', async () => {
    await mountFixture();
    const first = trigger('motion-first');
    const second = trigger('motion-second');

    enter(first);
    const host = await waitForPositionedTooltip('굵게');
    const presence = host.querySelector<HTMLElement>('[data-tooltip-presence]');
    const intro = presence?.getAnimations()[0];
    expect(intro).toBeDefined();
    if (!intro) throw new Error('Expected an intro animation');
    intro.pause();
    intro.currentTime = Number(intro.effect?.getTiming().duration ?? 0) / 2;
    await frames(1);

    leave(first);
    await vi.waitFor(() => expect(presence?.getAnimations()[0]).not.toBe(intro));
    const outro = presence?.getAnimations()[0];
    expect(outro).toBeDefined();
    if (!outro) throw new Error('Expected an outro animation');
    outro.pause();
    outro.currentTime = Number(outro.effect?.getTiming().duration ?? 0) / 2;
    await frames(1);
    const opacityBeforeReentry = Number.parseFloat(getComputedStyle(presence as HTMLElement).opacity);

    enter(second);
    const opacityAfterReentry = Number.parseFloat(getComputedStyle(presence as HTMLElement).opacity);
    expect(opacityAfterReentry).toBeCloseTo(opacityBeforeReentry, 2);
    await waitForTooltip('기울임');
    const returnAnimation = presence?.getAnimations()[0];
    expect(returnAnimation).toBeDefined();
    if (!returnAnimation) throw new Error('Expected a return animation');

    returnAnimation.pause();
    const duration = Number(returnAnimation.effect?.getTiming().duration ?? 0);
    returnAnimation.currentTime = returnAnimation.playbackRate < 0 ? 0 : duration;
    await frames(1);
    expect(getComputedStyle(presence as HTMLElement).opacity).toBe('1');
  });

  it('settles shared transitions immediately when reduced motion is requested', async () => {
    await mountFixture({ reducedMotion: true });
    const first = trigger('motion-first');
    const second = trigger('motion-second');

    enter(first);
    const host = await waitForTooltip('굵게');
    await vi.waitFor(() => expect(host.style.visibility).toBe('visible'));
    leave(first);
    enter(second);
    await waitForTooltip('기울임');
    await new Promise((resolve) => setTimeout(resolve, 60));

    expect(host.dataset.tooltipMotion).toBe('idle');
    expect(host.querySelector('[data-tooltip-content="outgoing"]')).toBeNull();
    expect(host.getAnimations({ subtree: true })).toHaveLength(0);
  });

  it('moves the shared host into the next overlay container with reduced motion', async () => {
    await mountFixture({ reducedMotion: true });
    const first = trigger('motion-first');
    const dialogAnchor = wrapperAnchor('motion-dialog');
    const dialog = dialogAnchor.closest('dialog');

    enter(first);
    const host = await waitForTooltip('굵게');
    await vi.waitFor(() => expect(host.style.visibility).toBe('visible'));
    leave(first);
    enter(dialogAnchor);
    await waitForTooltip('Dialog tooltip');

    expect(host.parentElement).toBe(dialog);
    expect(host.getAnimations({ subtree: true })).toHaveLength(0);
  });

  it('keeps the rendered tooltip stable when the far target updates during fade-out', async () => {
    await mountFixture();
    const first = trigger('motion-first');
    const far = trigger('motion-far');

    enter(first);
    const host = await waitForTooltip('굵게');
    await vi.waitFor(() => expect(host.style.visibility).toBe('visible'));
    leave(first);
    enter(far);
    await frames(1);
    expect(host.dataset.tooltipMotion).toBe('crossfade');
    const fadeOut = host.getAnimations().find((animation) => {
      const opacities = (animation.effect as KeyframeEffect | null)?.getKeyframes().map(({ opacity }) => opacity);
      return opacities?.[0] === '1' && opacities.at(-1) === '0';
    });
    expect(fadeOut).toBeDefined();
    fadeOut?.pause();
    trigger('motion-far-update').click();
    await tick();
    await frames(1);

    expect(host.textContent).toContain('굵게');
    expect(host.textContent).not.toContain('Updated far tooltip');
    fadeOut?.finish();
    await waitForTooltip('Updated far tooltip');
  });

  it('uses whole-tooltip crossfade when wrapper surface styling changes', async () => {
    await mountFixture();
    const first = wrapperAnchor('motion-wrapper-styled-first');
    const second = wrapperAnchor('motion-wrapper-styled-second');

    enter(first);
    const host = await waitForTooltip('Styled wrapper first');
    await vi.waitFor(() => expect(host.style.visibility).toBe('visible'));
    leave(first);
    enter(second);
    await frames(1);

    expect(host.dataset.tooltipMotion).toBe('crossfade');
    expect(host.textContent).toContain('Styled wrapper first');
  });

  it('moves the host when retargeting across containers before the first position is published', async () => {
    await mountFixture();
    const first = trigger('motion-first');
    const dialogAnchor = wrapperAnchor('motion-dialog');
    const dialog = dialogAnchor.closest('dialog');

    enter(first);
    leave(first);
    enter(dialogAnchor);
    const host = await waitForTooltip('Dialog tooltip');

    expect(host.parentElement).toBe(dialog);
  });

  it('does not reveal an intermediate target when hover retargets before its switch is presented', async () => {
    await mountFixture();
    const first = trigger('motion-first');
    const second = trigger('motion-second');
    const far = trigger('motion-far');

    enter(first);
    const host = await waitForPositionedTooltip('굵게');

    leave(first);
    enter(second);
    leave(second);
    enter(far);
    await frames(1);

    expect(host.dataset.tooltipMotion).toBe('crossfade');
    expect(host.textContent).toContain('굵게');
    expect(host.textContent).not.toContain('기울임');
    await waitForTooltip('Motion far');
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('idle'));
  });

  it('promotes a retarget that wins before the first anchor position is published', async () => {
    await mountFixture();
    const first = trigger('motion-first');
    const second = trigger('motion-second');
    const differentSide = trigger('motion-side');

    enter(first);
    leave(first);
    enter(second);
    const host = await waitForTooltip('기울임');

    leave(second);
    enter(differentSide);
    await frames(1);

    expect(host.dataset.tooltipMotion).toBe('crossfade');
    expect(host.textContent).toContain('기울임');
    expect(host.textContent).not.toContain('굵게');
    await waitForTooltip('Motion side');
  });

  it('crossfades when the resolved side or overlay container changes', async () => {
    await mountFixture();
    const first = trigger('motion-first');

    enter(first);
    const host = await waitForPositionedTooltip('굵게');

    leave(first);
    const sideAnchor = trigger('motion-side');
    enter(sideAnchor);
    await frames(1);
    expect(host.dataset.tooltipMotion).toBe('crossfade');
    await waitForTooltip('Motion side');
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('idle'));

    leave(sideAnchor);
    const dialogAnchor = wrapperAnchor('motion-dialog');
    enter(dialogAnchor);
    await frames(1);
    expect(host.dataset.tooltipMotion).toBe('crossfade');
    await waitForTooltip('Dialog tooltip');
    expect(host.parentElement).toBe(dialogAnchor.closest('dialog'));
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('idle'));
  });

  it('tracks scrolling immediately, stays visible when partially clipped, and closes when fully clipped', async () => {
    await mountFixture();
    const anchor = trigger('motion-first');
    const region = trigger('motion-scroll-region');

    enter(anchor);
    const host = await waitForPositionedTooltip('굵게');
    const initialAnchorTop = anchor.getBoundingClientRect().top;
    const initialTooltipTop = host.getBoundingClientRect().top;

    region.scrollTop = 20;
    region.dispatchEvent(new Event('scroll'));
    await vi.waitFor(() => expect(anchor.getBoundingClientRect().top).toBeLessThan(initialAnchorTop));
    await vi.waitFor(() => {
      const anchorDelta = anchor.getBoundingClientRect().top - initialAnchorTop;
      const tooltipDelta = host.getBoundingClientRect().top - initialTooltipTop;
      expect(tooltipDelta).toBeCloseTo(anchorDelta, 0);
    });
    expect(host.dataset.tooltipMotion).toBe('idle');

    region.scrollTop = 70;
    region.dispatchEvent(new Event('scroll'));
    await frames(2);
    expect(tooltip()).toBe(host);

    region.scrollTop = 110;
    region.dispatchEvent(new Event('scroll'));
    await vi.waitFor(() => expect(tooltip()).toBeNull());
  });

  it('cancels nearby travel when scrolling moves the active anchor and closes if it detaches', async () => {
    await mountFixture();
    const first = trigger('motion-first');
    const second = trigger('motion-second');
    const region = trigger('motion-scroll-region');

    enter(first);
    const host = await waitForPositionedTooltip('굵게');
    leave(first);
    enter(second);
    await waitForTooltip('기울임');
    expect(host.dataset.tooltipMotion).toBe('travel');

    region.scrollTop = 20;
    region.dispatchEvent(new Event('scroll'));
    await vi.waitFor(() => expect(host.dataset.tooltipMotion).toBe('idle'));
    const anchorRect = second.getBoundingClientRect();
    const hostRect = host.getBoundingClientRect();
    expect(hostRect.top).toBeGreaterThanOrEqual(anchorRect.bottom);

    second.remove();
    await vi.waitFor(() => expect(tooltip()).toBeNull());
  });
});
