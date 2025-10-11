import { getContext, setContext } from 'svelte';

export type WidgetType = 'characterCount' | 'characterCountChange' | 'postRelatedNote';

export type WidgetInstance = {
  id: string;
  name: string;
  data: Record<string, unknown>;
  order: string;
};

type WidgetState = {
  widgets: WidgetInstance[];
};

const key: unique symbol = Symbol('WidgetContext');

export class WidgetContext {
  state = $state<WidgetState>({
    widgets: [],
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
