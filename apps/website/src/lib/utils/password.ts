const DISALLOWED_RE = /[^\u{21}-\u{7E}]/gu;

export const sanitizePasswordInput = (element: HTMLInputElement) => {
  const value = element.value;
  const sanitized = value.replaceAll(DISALLOWED_RE, '');

  if (sanitized !== value) {
    const caret = element.selectionStart ?? value.length;
    const sanitizedCaret = value.slice(0, caret).replaceAll(DISALLOWED_RE, '').length;

    element.value = sanitized;
    element.setSelectionRange(sanitizedCaret, sanitizedCaret);
  }

  return sanitized;
};
