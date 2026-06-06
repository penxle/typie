export type ContextMenuCapabilityInput = {
  isSelectionCollapsed: boolean;
  readOnly: boolean;
  protectContent: boolean;
};

export type ContextMenuCapabilityState = {
  copyDisabled: boolean;
  cutDisabled: boolean;
  pasteDisabled: boolean;
  selectAllDisabled: boolean;
};

export const getContextMenuCapabilityState = ({
  isSelectionCollapsed,
  readOnly,
  protectContent,
}: ContextMenuCapabilityInput): ContextMenuCapabilityState => ({
  copyDisabled: isSelectionCollapsed || (readOnly && protectContent),
  cutDisabled: isSelectionCollapsed || readOnly,
  pasteDisabled: readOnly,
  selectAllDisabled: false,
});
