import CharacterCountChangeWidget from './CharacterCountChangeWidget.svelte';
import CharacterCountWidget from './CharacterCountWidget.svelte';
import OnboardingWidget from './OnboardingWidget.svelte';
import PostRelatedNoteWidget from './PostRelatedNoteWidget.svelte';
import type { Editor } from '@tiptap/core';
import type { Ref } from '@typie/ui/utils';
import type { Component } from 'svelte';
import type { Editor_Widget_CharacterCountChangeWidget_post, Editor_Widget_PostRelatedNoteWidget_post } from '$graphql';
import type { WidgetType } from '../widget-context.svelte';

export { default as CharacterCountChangeWidget } from './CharacterCountChangeWidget.svelte';
export { default as CharacterCountWidget } from './CharacterCountWidget.svelte';
export { default as OnboardingWidget } from './OnboardingWidget.svelte';
export { default as PostRelatedNoteWidget } from './PostRelatedNoteWidget.svelte';

export type WidgetComponentProps = {
  widgetId: string;
  palette?: boolean;
  disabled?: boolean;
  editMode?: boolean;
  editor?: Ref<Editor>;
  $post?: Editor_Widget_CharacterCountChangeWidget_post | Editor_Widget_PostRelatedNoteWidget_post;
  data?: Record<string, unknown>;
};

export type WidgetComponent = Component<WidgetComponentProps>;

export const WIDGET_COMPONENTS: Record<WidgetType, WidgetComponent> = {
  characterCount: CharacterCountWidget as WidgetComponent,
  characterCountChange: CharacterCountChangeWidget as WidgetComponent,
  postRelatedNote: PostRelatedNoteWidget as WidgetComponent,
  onboarding: OnboardingWidget as WidgetComponent,
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
];

export const WIDGET_CATEGORIES: { id: WidgetCategory; name: string }[] = [{ id: 'writing', name: '위젯' }];
