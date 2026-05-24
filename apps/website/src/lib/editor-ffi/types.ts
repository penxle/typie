import type { EditorEvent, InteractiveHit } from '@typie/editor-ffi/browser';
import type { Component } from 'svelte';
import type { Editor } from './editor.svelte';

export type ImageStage = 'empty' | 'uploading' | 'resolving' | 'ready';
export type FileStage = 'empty' | 'uploading' | 'resolving' | 'ready';

export type EditorEventListener<K extends EditorEvent['type']> = (editor: Editor, event: Extract<EditorEvent, { type: K }>) => void;

export type EditorEventHandler<E extends Element, T extends Event> = (editor: Editor, event: T & { currentTarget: E }) => void;

export type ImageAsset = {
  id: string;
  url: string;
  originalUrl: string;
  width: number;
  height: number;
  placeholder: string;
};

export type EmbedAsset = {
  id: string;
  url: string;
  title: string | null;
  description: string | null;
  thumbnailUrl: string | null;
  html: string | null;
};

export type ArchivedAsset = {
  id: string;
  content: string | null;
};

export type ContextMenuItem = {
  label: string;
  icon?: Component;
  variant?: 'default' | 'danger';
  onclick: () => void | Promise<void>;
};

export type ContextMenuSource = 'mouse' | 'touch';

export type ContextMenuPlacement = 'bottom-start' | 'top' | 'bottom';

export type ContextMenuContributorContext = {
  hit: InteractiveHit | undefined;
  clientX: number;
  clientY: number;
};

export type ContextMenuContributor = (ctx: ContextMenuContributorContext) => ContextMenuItem[];

export type FileAsset = {
  id: string;
  name: string;
  size: string;
  url: string;
};
