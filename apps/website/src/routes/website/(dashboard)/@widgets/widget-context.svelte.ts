import { getContext, setContext } from 'svelte';
import type { Editor } from '@tiptap/core';
import type { Ref } from '@typie/ui/utils';
import type { Editor_Widget_CharacterCountChangeWidget_post, Editor_Widget_PostRelatedNoteWidget_post } from '$graphql';

export type WidgetType = 'characterCount' | 'characterCountChange' | 'postRelatedNote' | 'onboarding';

export type WidgetInstance = {
  id: string;
  name: string;
  data: Record<string, unknown>;
  order: string;
};

type WidgetState = {
  widgets: WidgetInstance[];
};

type WidgetEnvironment = {
  editMode: boolean;
  palette: boolean;
  editor?: Ref<Editor>;
  $post?: (Editor_Widget_CharacterCountChangeWidget_post & Editor_Widget_PostRelatedNoteWidget_post) | undefined;
};

const key: unique symbol = Symbol('WidgetContext');

export class WidgetContext {
  state = $state<WidgetState>({
    widgets: [],
  });

  env = $state<WidgetEnvironment>({
    editMode: false,
    palette: false,
    editor: undefined,
    $post: undefined,
  });

  createWidget?: (type: WidgetType, index?: number) => Promise<void>;
  deleteWidget?: (id: string) => Promise<void>;
  updateWidget?: (id: string, data: Record<string, unknown>) => Promise<void>;
  moveWidget?: (widgetId: string, targetIndex: number) => Promise<void>;
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
