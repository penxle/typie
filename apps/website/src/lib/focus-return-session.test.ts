import { afterEach, describe, expect, it, vi } from 'vitest';
import { FocusReturnSession } from './focus-return-session';

afterEach(() => {
  document.body.replaceChildren();
});

function appendTo<T extends Node>(parent: ParentNode, child: T): T {
  parent.append(child);
  return child;
}

describe('FocusReturnSession', () => {
  it('restores the exact target without scrolling', () => {
    const target = appendTo(document.body, document.createElement('input'));
    const other = appendTo(document.body, document.createElement('button'));
    target.focus();
    const focus = vi.spyOn(target, 'focus');
    const session = FocusReturnSession.capture(target);
    other.focus();

    expect(session?.restore()).toBe(true);
    expect(document.activeElement).toBe(target);
    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
    expect(session?.restore()).toBe(false);
  });

  it('does not capture document roots, non-HTML elements, or another document', () => {
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    const foreignDocument = document.implementation.createHTMLDocument();
    const foreign = appendTo(foreignDocument.body, foreignDocument.createElement('button'));

    expect(FocusReturnSession.capture(null)).toBeNull();
    expect(FocusReturnSession.capture(document.body)).toBeNull();
    expect(FocusReturnSession.capture(document.documentElement)).toBeNull();
    expect(FocusReturnSession.capture(svg)).toBeNull();
    expect(FocusReturnSession.capture(foreign)).toBeNull();
  });

  it('consumes a disconnected target without retrying later', () => {
    const target = appendTo(document.body, document.createElement('button'));
    const session = FocusReturnSession.capture(target);
    target.remove();

    expect(session?.restore()).toBe(false);
    document.body.append(target);
    expect(session?.restore()).toBe(false);
  });

  it('preserves an input selection range', () => {
    const target = appendTo(document.body, document.createElement('input'));
    const other = appendTo(document.body, document.createElement('button'));
    target.value = 'focus return';
    target.focus();
    target.setSelectionRange(2, 8);
    const session = FocusReturnSession.capture(target);
    other.focus();

    expect(session?.restore()).toBe(true);
    expect([target.selectionStart, target.selectionEnd]).toEqual([2, 8]);
  });

  it('restores conditionally only while the region owns focus', () => {
    const target = appendTo(document.body, document.createElement('button'));
    const region = appendTo(document.body, document.createElement('div'));
    const inside = appendTo(region, document.createElement('input'));
    const outside = appendTo(document.body, document.createElement('button'));

    const restoring = FocusReturnSession.capture(target);
    inside.focus();
    expect(restoring?.restoreIfFocusWithin(region)).toBe(true);
    expect(document.activeElement).toBe(target);

    const discarding = FocusReturnSession.capture(target);
    outside.focus();
    expect(discarding?.restoreIfFocusWithin(region)).toBe(false);
    inside.focus();
    expect(discarding?.restore()).toBe(false);
  });

  it('discard is one-shot and idempotent', () => {
    const target = appendTo(document.body, document.createElement('button'));
    const session = FocusReturnSession.capture(target);
    session?.discard();
    session?.discard();

    expect(session?.restore()).toBe(false);
  });
});
