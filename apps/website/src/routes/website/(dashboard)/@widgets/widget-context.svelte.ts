import { getContext, setContext } from 'svelte';
import type { Editor_Widget_CharacterCountChangeWidget_document$key, Editor_Widget_DocumentRelatedNoteWidget_document$key } from '$mearie';
import type { RegisteredEditor } from '../[slug]/@pane/editor-registry.svelte';

export type WidgetType = 'characterCount' | 'characterCountChange' | 'postRelatedNote' | 'onboarding' | 'timer';

export type WidgetPosition = {
  top?: string;
  left?: string;
  bottom?: string;
  right?: string;
};

type WidgetEnvironment = {
  editMode: boolean;
  palette: boolean;
  editor?: RegisteredEditor;
  document$key?: (Editor_Widget_CharacterCountChangeWidget_document$key & Editor_Widget_DocumentRelatedNoteWidget_document$key) | undefined;
};

const key: unique symbol = Symbol('WidgetContext');

export class WidgetContext {
  env = $state<WidgetEnvironment>({
    editMode: false,
    palette: false,
    editor: undefined,
    document$key: undefined,
  });

  createWidget?: (type: WidgetType, via: string, index?: number) => Promise<void>;
  deleteWidget?: (id: string, via: string) => Promise<void>;
  updateWidget?: (id: string, data: Record<string, unknown>) => Promise<void>;
  moveWidgetInGroup?: (widgetId: string, targetIndex: number) => Promise<void>;
  moveWidgetToFreePosition?: (widgetId: string, position: WidgetPosition) => Promise<void>;
}

export const setupWidgetContext = () => {
  const context = new WidgetContext();
  setContext(key, context);
  return context;
};

export const getWidgetContext = (): WidgetContext => {
  const context = getContext<WidgetContext>(key);
  if (!context) {
    throw new Error('WidgetContext not found');
  }
  return context;
};
