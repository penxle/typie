import type { AttachmentPlaceholderKind, EditorEvent, InteractiveHit } from '@typie/editor-ffi/browser';
import type { Component } from 'svelte';
import type { Editor } from './editor.svelte';

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

// 서버가 아직 완결하지 않은 asset의 메타데이터(sync `asset-state`의 pending meta와 동형).
export type ServerPendingAsset = {
  kind: 'image' | 'file';
  name: string;
  size: number;
};

// 아직 준비되지 않은 id의 서버 상태. `missing`은 "지금은 해석 불가"일 뿐 부재의 확정이 아니다.
export type AssetResolution = { state: 'pending'; meta: ServerPendingAsset } | { state: 'missing' };

// 재시도 방법이 단계마다 다르므로 실패한 단계를 기록한다.
//  'transfer'           → asset-failed로 lease가 지워졌을 수 있다 → abandon 선행 후 새 nonce 재예약
//  'finalize-rejected'  → 서버가 claim을 해제했다 → 같은 nonce로 finalize 재호출
//  'finalize-retryable' → lease 그대로(인프라·timeout) → 같은 nonce로 finalize 재호출
export type LocalUploadFailureStage = 'transfer' | 'finalize-rejected' | 'finalize-retryable';

// 이 클라이언트가 직접 진행 중인 업로드. **assetId 키**이며 바이트 진행률은 없다(불확정 인디케이터).
export type LocalUpload = {
  kind: AttachmentPlaceholderKind;
  nodeId: string;
  file: File;
  nonce: string;
  previewUrl?: string;
  width?: number;
  height?: number;
  name: string;
  size: number;
  phase: 'active' | 'failed';
  failedStage?: LocalUploadFailureStage;
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
