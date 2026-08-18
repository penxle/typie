import { app, WebContentsView } from 'electron';
import { preloadPath } from './window-manager';

const ERR_ABORTED = -3;

export type TabViewHandlers = {
  onTitle: (title: string) => void;
  onLoading: (loading: boolean) => void;
  onUrl: (url: string) => void;
  onFailed: (url: string, code: number) => void;
  onCrashed: (url: string) => void;
};

export const createTabView = (handlers: TabViewHandlers) => {
  const view = new WebContentsView({
    webPreferences: {
      preload: preloadPath('tab'),
      sandbox: true,
      contextIsolation: true,
      spellcheck: false,
      additionalArguments: [`--typie-app-version=${app.getVersion()}`],
    },
  });
  const wc = view.webContents;
  wc.setVisualZoomLevelLimits(1, 1);
  wc.on('page-title-updated', (_event, title) => handlers.onTitle(title));
  wc.on('did-start-loading', () => handlers.onLoading(true));
  wc.on('did-stop-loading', () => handlers.onLoading(false));
  wc.on('did-navigate', (_event, url) => handlers.onUrl(url));
  wc.on('did-navigate-in-page', (_event, url, isMainFrame) => {
    if (isMainFrame) handlers.onUrl(url);
  });
  wc.on('did-fail-load', (_event, errorCode, _description, validatedURL, isMainFrame) => {
    if (isMainFrame && errorCode !== ERR_ABORTED) handlers.onFailed(validatedURL, errorCode);
  });
  wc.on('render-process-gone', () => handlers.onCrashed(wc.getURL()));
  return view;
};
