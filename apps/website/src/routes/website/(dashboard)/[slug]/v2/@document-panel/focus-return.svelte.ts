import { createStableContext } from '@typie/ui/context/stable';
import { FocusReturnSession } from '$lib/focus-return-session';

type DocumentPanelFocusReturn = {
  capture: (target: EventTarget | null) => void;
  restore: () => void;
  discard: () => void;
};

const [getDocumentPanelFocusReturn, setDocumentPanelFocusReturn] = createStableContext<DocumentPanelFocusReturn>(
  'document-panel.DocumentPanelFocusReturn',
);

export { getDocumentPanelFocusReturn };

export function setupDocumentPanelFocusReturn(): DocumentPanelFocusReturn {
  let session: FocusReturnSession | null = null;

  const controller: DocumentPanelFocusReturn = {
    capture(target) {
      session ??= FocusReturnSession.capture(target);
    },
    restore() {
      const captured = session;
      session = null;
      captured?.restore();
    },
    discard() {
      const captured = session;
      session = null;
      captured?.discard();
    },
  };

  // TODO: Let Workbench layers carry this session when a direct transition replaces
  // the document panel with another auxiliary layer.
  return setDocumentPanelFocusReturn(controller);
}
