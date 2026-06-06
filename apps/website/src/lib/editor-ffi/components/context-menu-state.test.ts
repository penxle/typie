import { describe, expect, it } from 'vitest';
import { getContextMenuCapabilityState } from './context-menu-state';

describe('getContextMenuCapabilityState', () => {
  it('enables copy and cut for editable range selections', () => {
    expect(
      getContextMenuCapabilityState({
        isSelectionCollapsed: false,
        readOnly: false,
        protectContent: false,
      }),
    ).toEqual({
      copyDisabled: false,
      cutDisabled: false,
      pasteDisabled: false,
      selectAllDisabled: false,
    });
  });

  it('disables copy and cut when selection is collapsed', () => {
    expect(
      getContextMenuCapabilityState({
        isSelectionCollapsed: true,
        readOnly: false,
        protectContent: false,
      }),
    ).toEqual({
      copyDisabled: true,
      cutDisabled: true,
      pasteDisabled: false,
      selectAllDisabled: false,
    });
  });

  it('disables cut and paste in read-only mode', () => {
    expect(
      getContextMenuCapabilityState({
        isSelectionCollapsed: false,
        readOnly: true,
        protectContent: false,
      }),
    ).toEqual({
      copyDisabled: false,
      cutDisabled: true,
      pasteDisabled: true,
      selectAllDisabled: false,
    });
  });

  it('disables copy for protected read-only content', () => {
    expect(
      getContextMenuCapabilityState({
        isSelectionCollapsed: false,
        readOnly: true,
        protectContent: true,
      }),
    ).toEqual({
      copyDisabled: true,
      cutDisabled: true,
      pasteDisabled: true,
      selectAllDisabled: false,
    });
  });
});
