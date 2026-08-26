import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import TooltipTestRoot from './tooltip-test-root.svelte';
import type { ComponentProps } from 'svelte';

type Props = ComponentProps<typeof TooltipTestRoot>;

class ReactiveProps<T extends object> {
  value: T = $state() as T;

  constructor(value: T) {
    this.value = value;
  }
}

const reactiveProps = <T extends object>(value: T): T => new ReactiveProps(value).value;

let component: Record<string, unknown> | undefined;
let props: Props;
let originalAnimateDescriptor: PropertyDescriptor | undefined;
let originalGetBoundingClientRectDescriptor: PropertyDescriptor | undefined;

const defaults = (): Props => ({
  firstMessage: 'First tooltip',
  firstDelay: 500,
  firstKeepOnClick: false,
  firstPlacement: 'bottom',
  firstArrow: true,
  secondMessage: 'Second tooltip',
  secondDelay: 500,
  thirdMessage: 'Third tooltip',
  thirdDelay: 0,
  wrapperMessage: 'Wrapper tooltip',
  wrapperEnabled: true,
  wrapperKeepShowing: false,
  wrapperUsesSnippet: false,
  wrapperPlacement: 'bottom',
  browserLayout: false,
});

const mountFixture = async (overrides: Partial<Props> = {}) => {
  props = reactiveProps({ ...defaults(), ...overrides });
  component = mount(TooltipTestRoot, { target: document.body, props });
  await tick();
};

const unmountFixture = async () => {
  if (component) await unmount(component);
  component = undefined;
  await tick();
};

const trigger = (testId: string): HTMLElement => {
  const element = document.querySelector<HTMLElement>(`[data-testid="${testId}"]`);
  if (!element) throw new Error(`Missing trigger: ${testId}`);
  return element;
};

const wrapperAnchor = (): HTMLElement => {
  const anchor = trigger('wrapper').parentElement;
  if (!(anchor instanceof HTMLElement)) throw new Error('Missing wrapper anchor');
  return anchor;
};

const tooltipElement = () => document.querySelector<HTMLElement>('[role="tooltip"]');
const tooltipElements = () => [...document.querySelectorAll<HTMLElement>('[role="tooltip"]')];

const enter = (element: HTMLElement) => {
  element.dispatchEvent(new Event('pointerenter'));
  element.dispatchEvent(new Event('mouseenter'));
};

const leave = (element: HTMLElement) => {
  element.dispatchEvent(new Event('pointerleave'));
  element.dispatchEvent(new Event('mouseleave'));
};

const settle = async () => {
  for (let cycle = 0; cycle < 3; cycle++) {
    await tick();
    await vi.advanceTimersByTimeAsync(0);
  }
  await tick();
};

const advance = async (milliseconds: number) => {
  await vi.advanceTimersByTimeAsync(milliseconds);
  await settle();
};

const finishLastAnimation = async () => {
  const animation = vi.mocked(Element.prototype.animate).mock.results.at(-1)?.value as Animation | undefined;
  animation?.onfinish?.call(animation, {} as AnimationPlaybackEvent);
  await settle();
};

const expectTooltip = (text: string) => {
  expect(tooltipElements()).toHaveLength(1);
  expect(tooltipElement()?.textContent).toContain(text);
};

beforeEach(() => {
  vi.useFakeTimers();
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe = vi.fn();
      unobserve = vi.fn();
      disconnect = vi.fn();
    },
  );
  originalAnimateDescriptor = Object.getOwnPropertyDescriptor(Element.prototype, 'animate');
  originalGetBoundingClientRectDescriptor = Object.getOwnPropertyDescriptor(Element.prototype, 'getBoundingClientRect');
  Object.defineProperties(Element.prototype, {
    animate: {
      configurable: true,
      value: vi.fn(
        () =>
          ({
            cancel: vi.fn(),
            currentTime: 0,
            effect: null,
            onfinish: null,
            playState: 'running',
          }) as unknown as Animation,
      ),
    },
    getBoundingClientRect: {
      configurable: true,
      value(this: Element) {
        if (this === document.documentElement || this === document.body) return new DOMRect(0, 0, 1024, 768);
        if (this.getAttribute('role') === 'tooltip') return new DOMRect(0, 0, 80, 24);
        return new DOMRect(100, 100, 80, 24);
      },
    },
  });
  Object.defineProperties(document.documentElement, {
    clientWidth: { configurable: true, value: 1024 },
    clientHeight: { configurable: true, value: 768 },
  });
  Object.defineProperties(document.body, {
    clientWidth: { configurable: true, value: 1024 },
    clientHeight: { configurable: true, value: 768 },
  });
});

afterEach(async () => {
  await unmountFixture();
  await vi.runOnlyPendingTimersAsync();
  if (originalAnimateDescriptor) Object.defineProperty(Element.prototype, 'animate', originalAnimateDescriptor);
  else delete (Element.prototype as Partial<Element>).animate;
  if (originalGetBoundingClientRectDescriptor) {
    Object.defineProperty(Element.prototype, 'getBoundingClientRect', originalGetBoundingClientRectDescriptor);
  } else {
    delete (Element.prototype as Partial<Element>).getBoundingClientRect;
  }
  Reflect.deleteProperty(document.documentElement, 'clientWidth');
  Reflect.deleteProperty(document.documentElement, 'clientHeight');
  Reflect.deleteProperty(document.body, 'clientWidth');
  Reflect.deleteProperty(document.body, 'clientHeight');
  vi.unstubAllGlobals();
  vi.useRealTimers();
  document.body.replaceChildren();
});

describe('tooltip opening delays', () => {
  it('opens the default action tooltip after 500ms', async () => {
    await mountFixture();
    enter(trigger('action-first'));

    await advance(499);
    expect(tooltipElement()).toBeNull();

    await advance(1);
    expectTooltip('First tooltip');
  });

  it('preserves an explicit 200ms opening delay', async () => {
    await mountFixture({ secondDelay: 200 });
    enter(trigger('action-second'));

    await advance(199);
    expect(tooltipElement()).toBeNull();

    await advance(1);
    expectTooltip('Second tooltip');
  });

  it('opens a 0ms action tooltip without advancing time', async () => {
    await mountFixture();
    enter(trigger('action-third'));
    await settle();

    expectTooltip('Third tooltip');
  });

  it('renders rich action content after its configured delay', async () => {
    await mountFixture();
    enter(trigger('action-snippet-trigger'));

    await advance(999);
    expect(tooltipElement()).toBeNull();

    await expect(advance(1)).resolves.toBeUndefined();
    expect(tooltipElement()?.querySelector('[data-testid="action-snippet"]')).not.toBeNull();
  });

  it('cancels and replaces a pending opening with the next trigger delay', async () => {
    await mountFixture({ secondDelay: 200 });
    const first = trigger('action-first');
    const second = trigger('action-second');

    enter(first);
    await advance(300);
    leave(first);
    enter(second);

    await advance(199);
    expect(tooltipElement()).toBeNull();

    await advance(1);
    expectTooltip('Second tooltip');
  });

  it('cannot reopen from a pending trigger after that trigger is destroyed', async () => {
    await mountFixture();
    enter(trigger('action-first'));
    await advance(200);

    await unmountFixture();
    await advance(500);

    expect(tooltipElement()).toBeNull();
  });

  it('does not restart a pending delay when only its content changes', async () => {
    await mountFixture();
    enter(trigger('action-first'));
    await advance(200);

    props.firstMessage = 'Updated tooltip';
    await settle();
    await advance(299);
    expect(tooltipElement()).toBeNull();

    await advance(1);
    expectTooltip('Updated tooltip');
  });

  it('opens a pending tooltip immediately when it becomes pinned', async () => {
    await mountFixture();
    enter(trigger('action-first'));
    await advance(200);

    props.firstForce = true;
    await settle();

    expectTooltip('First tooltip');
  });
});

describe('tooltip skip delay and shared host', () => {
  it('keeps the wrapper API immediate by default', async () => {
    await mountFixture();
    enter(wrapperAnchor());
    await settle();

    expectTooltip('Wrapper tooltip');
  });

  it('reuses the visible host and switches an adjacent trigger immediately', async () => {
    await mountFixture();
    const first = trigger('action-first');
    const second = trigger('action-second');

    enter(first);
    await advance(500);
    const firstHost = tooltipElement();
    expectTooltip('First tooltip');

    leave(first);
    enter(second);
    await settle();

    expectTooltip('Second tooltip');
    expect(tooltipElement()).toBe(firstHost);
  });

  it('shares one host when switching from an action tooltip to a wrapper tooltip', async () => {
    await mountFixture();
    const action = trigger('action-third');
    const wrapper = wrapperAnchor();

    enter(action);
    await settle();
    const firstHost = tooltipElement();
    leave(action);
    enter(wrapper);
    await settle();
    await finishLastAnimation();

    expectTooltip('Wrapper tooltip');
    expect(tooltipElement()).toBe(firstHost);
  });

  it('shares one host when switching from a wrapper tooltip to an action tooltip', async () => {
    await mountFixture();
    const wrapper = wrapperAnchor();
    const action = trigger('action-second');

    enter(wrapper);
    await settle();
    const firstHost = tooltipElement();
    leave(wrapper);
    enter(action);
    await settle();
    await finishLastAnimation();

    expectTooltip('Second tooltip');
    expect(tooltipElement()).toBe(firstHost);
  });

  it('preserves wrapper snippet content on the shared host', async () => {
    await mountFixture({ wrapperUsesSnippet: true });
    enter(wrapperAnchor());
    await settle();

    expect(document.querySelector('[data-testid="wrapper-snippet"]')).not.toBeNull();
    expectTooltip('Wrapper snippet');
  });

  it('reuses the visible host when the next trigger arrives at 79ms', async () => {
    await mountFixture();
    const first = trigger('action-third');
    const second = trigger('action-second');

    enter(first);
    await settle();
    const firstHost = tooltipElement();
    leave(first);
    await advance(79);
    enter(second);
    await settle();

    expectTooltip('Second tooltip');
    expect(tooltipElement()).toBe(firstHost);
  });

  it('closes at 80ms but opens a new host immediately from warm state', async () => {
    await mountFixture();
    const first = trigger('action-third');
    const second = trigger('action-second');

    enter(first);
    await settle();
    const firstHost = tooltipElement();
    leave(first);
    await advance(80);
    expect(tooltipElement()).toBeNull();

    enter(second);
    await settle();
    expectTooltip('Second tooltip');
    expect(tooltipElement()).not.toBe(firstHost);
  });

  it('opens immediately 299ms into the warm window', async () => {
    await mountFixture();
    const first = trigger('action-third');
    const second = trigger('action-second');

    enter(first);
    await settle();
    leave(first);
    await advance(80);
    await advance(299);
    enter(second);
    await settle();

    expectTooltip('Second tooltip');
  });

  it('restores the trigger delay when the 300ms warm window expires', async () => {
    await mountFixture();
    const first = trigger('action-third');
    const second = trigger('action-second');

    enter(first);
    await settle();
    leave(first);
    await advance(80);
    await advance(300);
    enter(second);

    await advance(499);
    expect(tooltipElement()).toBeNull();
    await advance(1);
    expectTooltip('Second tooltip');
  });

  it('retains warm state when the last visible trigger is destroyed', async () => {
    await mountFixture();
    enter(trigger('action-third'));
    await settle();
    expectTooltip('Third tooltip');

    await unmountFixture();
    await mountFixture();
    enter(trigger('action-second'));
    await settle();

    expectTooltip('Second tooltip');
  });
});

describe('tooltip eligibility and persistent visibility', () => {
  it('does not open or warm from an ineligible action message', async () => {
    await mountFixture({ firstMessage: null });
    const first = trigger('action-first');
    const second = trigger('action-second');

    enter(first);
    await advance(1000);
    expect(tooltipElement()).toBeNull();
    leave(first);
    enter(second);

    await advance(499);
    expect(tooltipElement()).toBeNull();
    await advance(1);
    expectTooltip('Second tooltip');
  });

  it('does not open or warm from an empty action message', async () => {
    await mountFixture({ firstMessage: '' });
    const first = trigger('action-first');
    const second = trigger('action-second');

    enter(first);
    await advance(1000);
    leave(first);
    enter(second);
    await advance(499);

    expect(tooltipElement()).toBeNull();
  });

  it('starts the normal delay when a hovered idle trigger becomes eligible', async () => {
    await mountFixture({ firstMessage: null });
    enter(trigger('action-first'));
    await advance(1000);

    props.firstMessage = 'First tooltip';
    await settle();
    await advance(499);
    expect(tooltipElement()).toBeNull();
    await advance(1);
    expectTooltip('First tooltip');
  });

  it('opens immediately when a hovered trigger becomes eligible during warm state', async () => {
    await mountFixture({ firstMessage: null });
    const first = trigger('action-first');
    const third = trigger('action-third');

    enter(third);
    await settle();
    leave(third);
    await advance(80);
    enter(first);
    props.firstMessage = 'First tooltip';
    await settle();

    expectTooltip('First tooltip');
  });

  it('closes a visible trigger that becomes ineligible and warms the next trigger', async () => {
    await mountFixture({ firstDelay: 0 });
    enter(trigger('action-first'));
    await settle();
    expectTooltip('First tooltip');

    props.firstMessage = null;
    await settle();
    expect(tooltipElement()).toBeNull();
    enter(trigger('action-second'));
    await settle();

    expectTooltip('Second tooltip');
  });

  it('does not open or warm a disabled wrapper', async () => {
    await mountFixture({ wrapperEnabled: false });
    const wrapper = wrapperAnchor();
    const second = trigger('action-second');

    enter(wrapper);
    await advance(1000);
    leave(wrapper);
    enter(second);
    await advance(499);

    expect(tooltipElement()).toBeNull();
  });

  it('pins force:true across pointer leave and rejects competing hover', async () => {
    await mountFixture({ firstForce: true });
    await settle();
    expectTooltip('First tooltip');

    leave(trigger('action-first'));
    enter(trigger('action-second'));
    await advance(500);

    expectTooltip('First tooltip');
  });

  it('suppresses hover when force is false', async () => {
    await mountFixture({ firstForce: false });
    enter(trigger('action-first'));
    await advance(1000);

    expect(tooltipElement()).toBeNull();
  });

  it('keeps an unpinned tooltip visible while its trigger remains hovered', async () => {
    await mountFixture({ firstForce: true });
    const first = trigger('action-first');
    enter(first);
    await settle();
    props.firstForce = undefined;
    await settle();

    expectTooltip('First tooltip');
  });

  it('closes an unpinned unhovered tooltip and warms the next trigger', async () => {
    await mountFixture({ firstForce: true });
    await settle();
    props.firstForce = undefined;
    await settle();
    expect(tooltipElement()).toBeNull();

    enter(trigger('action-second'));
    await settle();
    expectTooltip('Second tooltip');
  });

  it('keeps keepShowing wrapper content pinned across leave', async () => {
    await mountFixture({ wrapperKeepShowing: true });
    await settle();
    expectTooltip('Wrapper tooltip');

    leave(wrapperAnchor());
    enter(trigger('action-second'));
    await advance(500);
    expectTooltip('Wrapper tooltip');
  });
});

describe('tooltip click behavior', () => {
  it('closes a normal action tooltip on click', async () => {
    await mountFixture({ firstDelay: 0 });
    const first = trigger('action-first');
    enter(first);
    await settle();
    first.click();
    await settle();

    expect(tooltipElement()).toBeNull();
  });

  it('keeps an action tooltip open on click when keepOnClick is true', async () => {
    await mountFixture({ firstDelay: 0, firstKeepOnClick: true });
    const first = trigger('action-first');
    enter(first);
    await settle();
    first.click();
    await settle();

    expectTooltip('First tooltip');
  });
});
