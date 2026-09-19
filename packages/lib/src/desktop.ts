import type { DocumentSaveDocument, DocumentSaveState } from './document-save';

export type TabIcon = { icon: string; color: string | null };

export type DesktopZoomAction = 'in' | 'out' | 'reset';

export type DocumentDepartureReason = 'close' | 'quit' | 'reload' | 'logout' | 'login';
export type DocumentSaveDialogAction = 'cancel' | 'retry' | 'discard' | 'continue';
export type DocumentSaveRequest = {
  id: string;
  operationId: string;
  reason?: DocumentDepartureReason;
  phase: 'prepare' | 'check' | 'commit' | 'release' | 'capture';
  discard?: boolean;
};
export type DocumentSaveResult = {
  // Preload combines the renderer's session identity with its page/registration.
  generation?: string;
  status: 'protected' | 'pending' | 'failed' | 'unknown';
  documents: {
    id: string;
    sessionId: string;
    title: string;
    icon?: string;
    iconColor?: string;
    status?: DocumentSaveState;
    protectedChanges?: boolean;
  }[];
};

export type DocumentSaveDialogData = {
  id: string;
  title: string;
  description: string;
  required: boolean;
  cancelLabel: string;
  discardLabel: string;
  completed?: boolean;
  continueLabel?: string;
  documents: DocumentSaveDocument[];
};

export type DesktopBridgeListeners = {
  focus: () => void;
  preference: () => void;
  'document-save-recovered': () => void;
  'document-save-progress': (visible: boolean) => void;
  'zoom-shortcut': (action: DesktopZoomAction) => boolean;
};

export type TypieDesktopBridge = {
  version: string;
  platform: 'darwin' | 'win32';
  openExternal: (url: string) => Promise<void>;
  on: <Event extends keyof DesktopBridgeListeners>(event: Event, callback: DesktopBridgeListeners[Event]) => () => void;
  setTabIcon?: (icon: TabIcon) => void;
  openTab?: (url: string) => void;
  onDocumentSave?: (handler: (request: DocumentSaveRequest) => Promise<DocumentSaveResult>) => () => void;
  requestDocumentDeparture?: (reason: 'logout' | 'login') => Promise<void>;
};
