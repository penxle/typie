import { Extension, isiOS } from '@tiptap/core';
import { NodeSelection, Plugin } from '@tiptap/pm/state';
import { token } from '@typie/styled-system/tokens';
import { MultiNodeSelection } from './selection';

const createOverlayOfParagraph = (paragraphElement: HTMLElement) => {
  const overlay = document.createElement('div');
  overlay.style.position = 'fixed';
  overlay.setAttribute('aria-hidden', 'true');
  overlay.setAttribute('role', 'presentation');
  overlay.setAttribute('inert', '');
  const paragraphRect = paragraphElement.getBoundingClientRect();
  overlay.style.left = `${paragraphRect.left}px`;
  overlay.style.top = `${paragraphRect.top}px`;
  overlay.style.width = `${paragraphRect.width}px`;
  overlay.style.height = `${paragraphRect.height}px`;
  overlay.style.pointerEvents = 'none';
  overlay.style.zIndex = token('zIndex.editor') as string;
  overlay.style.overflow = 'hidden';
  return overlay;
};

const cloneTransparentParagraph = (paragraphElement: HTMLElement) => {
  const clonedParagraph = paragraphElement.cloneNode(true) as HTMLElement;
  clonedParagraph.style.margin = '0';
  clonedParagraph.style.position = 'relative';
  clonedParagraph.setAttribute('aria-hidden', 'true');
  clonedParagraph.setAttribute('role', 'presentation');

  const paragraphStyle = window.getComputedStyle(paragraphElement);

  const textElement = (paragraphElement.querySelector('[style*="font-size"]') as HTMLElement) || paragraphElement;
  const textStyle = window.getComputedStyle(textElement);

  clonedParagraph.style.fontSize = textStyle.fontSize;
  clonedParagraph.style.fontFamily = textStyle.fontFamily;
  clonedParagraph.style.fontWeight = textStyle.fontWeight;
  clonedParagraph.style.fontStyle = textStyle.fontStyle;
  clonedParagraph.style.fontVariant = textStyle.fontVariant;
  clonedParagraph.style.fontFeatureSettings = textStyle.fontFeatureSettings;
  clonedParagraph.style.fontKerning = textStyle.fontKerning;
  clonedParagraph.style.fontOpticalSizing = textStyle.fontOpticalSizing;
  clonedParagraph.style.fontSizeAdjust = textStyle.fontSizeAdjust;

  clonedParagraph.style.lineHeight = textStyle.lineHeight;
  clonedParagraph.style.letterSpacing = textStyle.letterSpacing;
  clonedParagraph.style.wordSpacing = textStyle.wordSpacing;
  clonedParagraph.style.textAlign = textStyle.textAlign;
  clonedParagraph.style.textAlignLast = textStyle.textAlignLast;
  clonedParagraph.style.textIndent = textStyle.textIndent;
  clonedParagraph.style.verticalAlign = textStyle.verticalAlign;

  clonedParagraph.style.textTransform = textStyle.textTransform;
  clonedParagraph.style.textDecoration = textStyle.textDecoration;
  clonedParagraph.style.textDecorationLine = textStyle.textDecorationLine;
  clonedParagraph.style.textDecorationStyle = textStyle.textDecorationStyle;
  clonedParagraph.style.textDecorationThickness = textStyle.textDecorationThickness;
  clonedParagraph.style.textUnderlineOffset = textStyle.textUnderlineOffset;
  clonedParagraph.style.textShadow = textStyle.textShadow;

  clonedParagraph.style.textRendering = textStyle.textRendering;

  clonedParagraph.style.whiteSpace = textStyle.whiteSpace;
  clonedParagraph.style.wordBreak = textStyle.wordBreak;
  clonedParagraph.style.overflowWrap = textStyle.overflowWrap;
  clonedParagraph.style.hyphens = textStyle.hyphens;
  clonedParagraph.style.tabSize = textStyle.tabSize;

  clonedParagraph.style.direction = textStyle.direction;
  clonedParagraph.style.writingMode = textStyle.writingMode;
  clonedParagraph.style.textOrientation = textStyle.textOrientation;

  clonedParagraph.style.textOverflow = textStyle.textOverflow;
  clonedParagraph.style.quotes = textStyle.quotes;
  clonedParagraph.style.unicodeBidi = textStyle.unicodeBidi;

  clonedParagraph.style.padding = paragraphStyle.padding;

  // NOTE: 모든 요소를 투명하게 만들기
  const allElements = clonedParagraph.querySelectorAll('*');
  allElements.forEach((el) => {
    if (el instanceof HTMLElement) {
      el.style.color = 'transparent';
      el.style.backgroundColor = 'transparent';
    }
  });
  clonedParagraph.style.color = 'transparent';
  clonedParagraph.style.backgroundColor = 'transparent';

  return clonedParagraph;
};

const getTextNodePairs = (paragraphElement: HTMLElement, clonedParagraph: HTMLElement) => {
  const textNodes: { original: Text; cloned: Text }[] = [];

  const originalWalker = document.createTreeWalker(paragraphElement, NodeFilter.SHOW_TEXT, null);
  const clonedWalker = document.createTreeWalker(clonedParagraph, NodeFilter.SHOW_TEXT, null);

  let originalNode = originalWalker.nextNode() as Text | null;
  let clonedNode = clonedWalker.nextNode() as Text | null;

  while (originalNode && clonedNode) {
    textNodes.push({ original: originalNode, cloned: clonedNode });
    originalNode = originalWalker.nextNode() as Text | null;
    clonedNode = clonedWalker.nextNode() as Text | null;
  }

  return textNodes;
};

const getStartOffset = (selectionRange: Range, node: Text) => {
  if (selectionRange.startContainer === node) {
    // 선택 범위가 이 텍스트 노드에서 시작하는 경우
    return selectionRange.startOffset;
  } else if (selectionRange.startContainer.compareDocumentPosition(node) & 4) {
    // 선택 범위가 이 텍스트 노드보다 앞에서 시작하는 경우
    // NOTE: DOCUMENT_POSITION_FOLLOWING = 4
    return 0;
  } else {
    // 선택 범위가 이 텍스트 노드보다 뒤에서 시작하는 경우. 이 경우 텍스트 노드는 선택되지 않음
    return null;
  }
};

const getEndOffset = (selectionRange: Range, node: Text) => {
  if (selectionRange.endContainer === node) {
    // 선택 범위가 이 텍스트 노드에서 끝나는 경우
    return selectionRange.endOffset;
  } else if (selectionRange.endContainer.compareDocumentPosition(node) & 2) {
    // 선택 범위가 이 텍스트 노드보다 뒤에서 끝나는 경우
    // NOTE: DOCUMENT_POSITION_PRECEDING = 2
    return node.textContent?.length || 0;
  } else {
    // 선택 범위가 이 텍스트 노드보다 앞에서 끝나는 경우. 이 경우 텍스트 노드는 선택되지 않음
    return null;
  }
};

const getTextFragment = (text: string, startOffset: number, endOffset: number) => {
  const beforeText = text.slice(0, Math.max(0, startOffset));
  const selectedText = text.slice(startOffset, endOffset);
  const afterText = text.slice(Math.max(0, endOffset));

  const fragment = document.createDocumentFragment();

  if (beforeText) {
    const beforeSpan = document.createElement('span');
    beforeSpan.style.color = 'transparent';
    beforeSpan.textContent = beforeText;
    fragment.append(beforeSpan);
  }

  if (selectedText) {
    const selectedSpan = document.createElement('span');
    selectedSpan.classList.add('ios-selection');
    selectedSpan.textContent = selectedText;
    fragment.append(selectedSpan);
  }

  if (afterText) {
    const afterSpan = document.createElement('span');
    afterSpan.style.color = 'transparent';
    afterSpan.textContent = afterText;
    fragment.append(afterSpan);
  }

  return fragment;
};

const createShadowTextOfSelectedLowContrastText = (
  paragraphElement: HTMLElement,
  selectionRange: Range | null,
  isNodeSelection: boolean,
) => {
  const overlay = createOverlayOfParagraph(paragraphElement);
  const clonedParagraph = cloneTransparentParagraph(paragraphElement);
  const textNodes = getTextNodePairs(paragraphElement, clonedParagraph);

  for (const { original, cloned } of textNodes) {
    if (!cloned.parentNode) continue;

    const clonedParent = cloned.parentElement;
    const isLowContrastText =
      clonedParent && (clonedParent.classList.contains('low-contrast-text') || clonedParent.closest('.low-contrast-text') !== null);

    if (!isLowContrastText) continue;

    if (isNodeSelection) {
      const span = document.createElement('span');
      span.classList.add('ios-selection');
      span.textContent = cloned.textContent;
      cloned.replaceWith(span);
      continue;
    }

    if (!selectionRange) continue;

    if (selectionRange.intersectsNode(original)) {
      const startOffset = getStartOffset(selectionRange, original);
      const endOffset = getEndOffset(selectionRange, original);

      if (startOffset === null || endOffset === null || startOffset >= endOffset) continue;

      const text = cloned.textContent || '';
      const fragment = getTextFragment(text, startOffset, endOffset);

      cloned.replaceWith(fragment);
    }
  }

  overlay.append(clonedParagraph);
  return overlay;
};

const repositionOverlay = (overlay: HTMLElement, source: HTMLElement) => {
  const rect = source.getBoundingClientRect();
  overlay.style.left = `${rect.left}px`;
  overlay.style.top = `${rect.top}px`;
  overlay.style.width = `${rect.width}px`;
  overlay.style.height = `${rect.height}px`;
};

type IOSSelectionStorage = {
  overlayElements: { overlay: HTMLElement; source: HTMLElement }[];
  eventListeners: {
    target: EventTarget;
    type: string;
    handler: EventListener;
    options?: AddEventListenerOptions | boolean;
  }[];
};

export const IOSSelection = Extension.create<unknown, IOSSelectionStorage>({
  name: 'ios-selection',

  addStorage() {
    return {
      overlayElements: [],
      eventListeners: [],
    };
  },

  onCreate() {
    this.storage.overlayElements = [];
    this.storage.eventListeners = [];
  },

  onDestroy() {
    this.storage.overlayElements.forEach(({ overlay }: { overlay: HTMLElement }) => overlay.remove());
    this.storage.overlayElements = [];

    this.storage.eventListeners.forEach(({ target, type, handler, options }) => {
      target.removeEventListener(type, handler, options);
    });
    this.storage.eventListeners = [];
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        view: () => ({
          update: (view) => {
            if (!isiOS() && window.__webview__?.platform !== 'ios') {
              return;
            }

            const { state } = view;
            const { doc, selection } = state;
            const { from, to } = selection;

            // NOTE: 기존 오버레이 제거
            this.storage.overlayElements.forEach(({ overlay }: { overlay: HTMLElement }) => overlay.remove());
            this.storage.overlayElements = [];

            this.storage.eventListeners.forEach(({ target, type, handler, options }) => {
              target.removeEventListener(type, handler, options);
            });
            this.storage.eventListeners = [];

            const windowSelection = window.getSelection();
            const selectionRange = windowSelection && windowSelection.rangeCount > 0 ? windowSelection.getRangeAt(0) : null;
            const isNodeSelection = selection instanceof NodeSelection || selection instanceof MultiNodeSelection;

            if (from === to || (!selectionRange && !isNodeSelection)) {
              return;
            }

            // NOTE: 선택 범위와 겹치는 모든 paragraph 중 .low-contrast-text 클래스를 포함한 것들
            const paragraphElements: HTMLElement[] = [];
            doc.nodesBetween(from, to, (node, pos) => {
              if (node.type.name === 'paragraph') {
                const domNode = view.nodeDOM(pos);
                if (
                  domNode instanceof HTMLElement &&
                  domNode.tagName === 'P' &&
                  !paragraphElements.includes(domNode) &&
                  (domNode.classList.contains('low-contrast-text') || domNode.querySelector('.low-contrast-text'))
                ) {
                  paragraphElements.push(domNode);
                }
              }
            });

            if (paragraphElements.length === 0) return;

            // NOTE: 모든 paragraph에 대해 오버레이 생성
            paragraphElements.forEach((paragraphElement) => {
              const overlay = createShadowTextOfSelectedLowContrastText(paragraphElement, selectionRange, isNodeSelection);
              document.body.append(overlay);
              this.storage.overlayElements.push({ overlay, source: paragraphElement });
            });

            // NOTE: viewport 변화 시 오버레이 재배치
            if (this.storage.overlayElements.length > 0) {
              let scrollTimeout: ReturnType<typeof setTimeout>;

              const onViewportChanged = () => {
                this.storage.overlayElements.forEach(({ overlay }: { overlay: HTMLElement }) => {
                  overlay.style.display = 'none';
                });

                clearTimeout(scrollTimeout);
                scrollTimeout = setTimeout(() => {
                  this.storage.overlayElements.forEach(({ overlay, source }: { overlay: HTMLElement; source: HTMLElement }) => {
                    repositionOverlay(overlay, source);
                    overlay.style.display = 'block';
                  });
                }, 150);
              };

              const addListener = (target: EventTarget, type: string, options?: AddEventListenerOptions | boolean) => {
                target.addEventListener(type, onViewportChanged, options);
                this.storage.eventListeners.push({ target, type, handler: onViewportChanged, options });
              };

              addListener(window, 'scroll', { capture: true, passive: true });
              addListener(window, 'resize', { capture: true, passive: true });
              addListener(window, 'orientationchange', { capture: true, passive: true });

              if (window.visualViewport) {
                addListener(window.visualViewport, 'resize');
                addListener(window.visualViewport, 'scroll', { passive: true });
              }
            }
          },
        }),
      }),
    ];
  },
});
