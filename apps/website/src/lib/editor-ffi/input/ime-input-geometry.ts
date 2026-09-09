const textStyleProperties = [
  'fontFamily',
  'fontSize',
  'fontWeight',
  'fontStyle',
  'fontStretch',
  'fontVariant',
  'fontFeatureSettings',
  'fontVariationSettings',
  'fontKerning',
  'fontOpticalSizing',
  'letterSpacing',
  'wordSpacing',
  'textRendering',
  'tabSize',
  'direction',
] as const;

export function syncImeInputScroll(input: HTMLTextAreaElement): void {
  const style = getComputedStyle(input);
  const mirror = document.createElement('div');
  Object.assign(mirror.style, {
    position: 'fixed',
    visibility: 'hidden',
    whiteSpace: 'pre',
    overflowWrap: 'normal',
    width: `${input.clientWidth}px`,
  });
  for (const property of textStyleProperties) {
    mirror.style[property] = style[property];
  }

  // A textarea does not expose its caret rectangle. Measure the same unwrapped
  // text with DOM layout, including tabs, fallback fonts, and bidirectional runs.
  const text = document.createTextNode(`${input.value}\u{200B}`);
  mirror.append(text);
  input.after(mirror);
  const range = document.createRange();
  const offset = input.selectionDirection === 'backward' ? input.selectionStart : input.selectionEnd;
  range.setStart(text, offset);
  range.collapse(true);
  const left = range.getBoundingClientRect().left - mirror.getBoundingClientRect().left;
  mirror.remove();

  // Programmatic selection changes do not reliably scroll the native caret into
  // view. Its x coordinate must coincide with the textarea's editor-caret anchor.
  input.scrollLeft = left;
}
