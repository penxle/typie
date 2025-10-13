import CharacterCountChangeWidget from './CharacterCountChangeWidget.svelte';
import CharacterCountWidget from './CharacterCountWidget.svelte';
import OnboardingWidget from './OnboardingWidget.svelte';
import PostRelatedNoteWidget from './PostRelatedNoteWidget.svelte';
import TimerWidget from './TimerWidget.svelte';
import type { Component } from 'svelte';
import type { WidgetType } from '../widget-context.svelte';

export { default as CharacterCountChangeWidget } from './CharacterCountChangeWidget.svelte';
export { default as CharacterCountWidget } from './CharacterCountWidget.svelte';
export { default as OnboardingWidget } from './OnboardingWidget.svelte';
export { default as PostRelatedNoteWidget } from './PostRelatedNoteWidget.svelte';
export { default as TimerWidget } from './TimerWidget.svelte';

export type WidgetComponentProps = {
  widgetId: string;
  data?: Record<string, unknown>;
};

export type WidgetComponent = Component<WidgetComponentProps>;

export const WIDGET_COMPONENTS: Record<WidgetType, Component<WidgetComponentProps>> = {
  characterCount: CharacterCountWidget,
  characterCountChange: CharacterCountChangeWidget,
  postRelatedNote: PostRelatedNoteWidget,
  onboarding: OnboardingWidget,
  timer: TimerWidget,
};

export type WidgetCategory = 'writing';

export type WidgetMetadata = {
  type: WidgetType;
  category: WidgetCategory;
  name: string;
};

export const WIDGET_METADATA: WidgetMetadata[] = [
  { type: 'characterCount', category: 'writing', name: '글자 수' },
  { type: 'characterCountChange', category: 'writing', name: '오늘의 기록' },
  { type: 'postRelatedNote', category: 'writing', name: '노트' },
  { type: 'timer', category: 'writing', name: '타이머' },
];

export const WIDGET_CATEGORIES: { id: WidgetCategory; name: string }[] = [{ id: 'writing', name: '위젯' }];
