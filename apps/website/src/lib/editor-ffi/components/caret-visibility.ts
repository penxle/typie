export type CaretVisibilityInput = {
  hasCursor: boolean;
  hasPoint: boolean;
  focused: boolean;
  readOnly: boolean;
};

export const isCaretVisible = ({ hasCursor, hasPoint, focused, readOnly }: CaretVisibilityInput): boolean =>
  hasCursor && hasPoint && focused && !readOnly;
