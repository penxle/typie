import pLimit from 'p-limit';
import { getImageDimensions, uploadFileAsFile, uploadImageFile } from './handlers/upload';
import type { AttachmentPlaceholderKind, InputModifiers } from '@typie/editor-ffi/browser';
import type { Editor, EditorContext } from './editor.svelte';

export type AttachmentImportItem = Readonly<{
  file: File;
  kind: AttachmentPlaceholderKind;
}>;

export type AttachmentImportFailureHandler = (item: AttachmentImportItem) => void;

type AttachmentImportTarget = {
  editor: Editor;
  item: AttachmentImportItem;
  nodeId: string;
  uploadId: string;
  removeOnFailure: boolean;
  onFailure: AttachmentImportFailureHandler;
};

type MappedAttachment = {
  item: AttachmentImportItem;
  nodeId: string;
  removeOnFailure: boolean;
};

export class EditorAttachmentImporter {
  readonly #ctx: EditorContext;
  readonly #limit = pLimit(5);

  constructor(ctx: EditorContext) {
    this.#ctx = ctx;
  }

  #editableEditor(): Editor | undefined {
    const editor = this.#ctx.editor;
    return editor !== undefined && this.#isEditorCurrent(editor) ? editor : undefined;
  }

  #isEditorCurrent(editor: Editor): boolean {
    return this.#ctx.editor === editor && !editor.destroyed && !editor.readOnly;
  }

  #isAvailable(editor: Editor, nodeId: string, kind: AttachmentPlaceholderKind): boolean {
    return this.#isEmptyPlaceholder(editor, nodeId, kind) && !this.#pendingMap(editor, kind).has(nodeId);
  }

  #isEmptyPlaceholder(editor: Editor, nodeId: string, kind: AttachmentPlaceholderKind): boolean {
    const data = editor.externalElements.find((element) => element.node === nodeId)?.data;
    return data?.type === kind && (data.id === undefined || data.id === '');
  }

  #pendingMap(editor: Editor, kind: AttachmentPlaceholderKind): Editor['inflightImages'] | Editor['inflightFiles'] {
    return kind === 'image' ? editor.inflightImages : editor.inflightFiles;
  }

  #collectReceipt(editor: Editor, requestId: string, enqueue: () => void): string[] | undefined {
    const matches: string[][] = [];
    const dispose = editor.on('attachment_placeholders_inserted', (_, event) => {
      if (event.request_id === requestId) matches.push(event.node_ids);
    });
    try {
      enqueue();
      editor.flush();
    } catch (err) {
      console.error('Failed to enqueue or flush attachment placeholder request:', err);
      return undefined;
    } finally {
      dispose();
    }
    if (matches.length === 0) {
      console.error('Attachment placeholder request emitted no matching receipt.', requestId);
      return undefined;
    }
    if (matches.length > 1) {
      console.error('Attachment placeholder request emitted duplicate matching receipts.', {
        requestId,
        receiptCount: matches.length,
      });
      return undefined;
    }
    return matches[0];
  }

  #isExactMapping(nodeIds: readonly string[], expectedCount: number): boolean {
    if (nodeIds.length !== expectedCount) {
      console.error('Attachment placeholder receipt count mismatch.', {
        expectedCount,
        actualCount: nodeIds.length,
      });
      return false;
    }
    if (new Set(nodeIds).size !== expectedCount) {
      console.error('Attachment placeholder receipt contains duplicate node IDs.', nodeIds);
      return false;
    }
    return true;
  }

  #reserveAndSchedule(editor: Editor, mapped: readonly MappedAttachment[], onFailure: AttachmentImportFailureHandler): boolean {
    if (!this.#isEditorCurrent(editor) || mapped.some(({ item, nodeId }) => !this.#isAvailable(editor, nodeId, item.kind))) {
      return false;
    }

    const targets = mapped.map<AttachmentImportTarget>(({ item, nodeId, removeOnFailure }) => ({
      editor,
      item,
      nodeId,
      uploadId: crypto.randomUUID(),
      removeOnFailure,
      onFailure,
    }));

    for (const target of targets) {
      if (target.item.kind === 'image') {
        editor.inflightImages.set(target.nodeId, { uploadId: target.uploadId, width: 0, height: 0 });
      } else {
        editor.inflightFiles.set(target.nodeId, {
          uploadId: target.uploadId,
          name: target.item.file.name,
          size: target.item.file.size,
        });
      }
    }

    for (const target of targets) {
      void this.#limit(() => this.#process(target)).catch((err: unknown) => {
        console.error('Unexpected attachment import worker failure:', err);
      });
    }
    return true;
  }

  async #process(target: AttachmentImportTarget): Promise<void> {
    if (target.item.kind === 'image') {
      await this.#processImage(target);
    } else {
      await this.#processFile(target);
    }
  }

  async #processImage(target: AttachmentImportTarget): Promise<void> {
    let objectUrl: string | undefined;
    try {
      if (!this.#isCurrent(target)) return;

      objectUrl = URL.createObjectURL(target.item.file);
      if (!this.#isCurrent(target)) return;
      target.editor.inflightImages.set(target.nodeId, { uploadId: target.uploadId, url: objectUrl, width: 0, height: 0 });

      const { width, height } = await getImageDimensions(objectUrl);
      if (!this.#isCurrent(target)) return;
      target.editor.inflightImages.set(target.nodeId, { uploadId: target.uploadId, url: objectUrl, width, height });

      const uploaded = await uploadImageFile(target.item.file);
      if (!this.#isCurrent(target)) return;
      target.editor.imageAssets.set(uploaded.id, uploaded);
      if (!this.#isCurrent(target)) return;
      target.editor.enqueue({
        type: 'node',
        op: {
          type: 'set_attr',
          id: target.nodeId,
          attr: {
            type: 'image',
            attr: { type: 'id', value: uploaded.id },
          },
        },
      });
      target.editor.flush();
    } catch {
      this.#handleFailure(target);
    } finally {
      this.#cleanup(target, objectUrl);
    }
  }

  async #processFile(target: AttachmentImportTarget): Promise<void> {
    try {
      if (!this.#isCurrent(target)) return;
      const uploaded = await uploadFileAsFile(target.item.file);
      if (!this.#isCurrent(target)) return;
      this.#ctx.fileAssets.set(uploaded.id, uploaded);
      if (!this.#isCurrent(target)) return;
      target.editor.enqueue({
        type: 'node',
        op: {
          type: 'set_attrs',
          id: target.nodeId,
          attrs: { type: 'file', id: uploaded.id },
        },
      });
      target.editor.flush();
    } catch {
      this.#handleFailure(target);
    } finally {
      this.#cleanup(target);
    }
  }

  #isCurrent(target: AttachmentImportTarget): boolean {
    return (
      this.#isEditorCurrent(target.editor) &&
      this.#isEmptyPlaceholder(target.editor, target.nodeId, target.item.kind) &&
      this.#pendingMap(target.editor, target.item.kind).get(target.nodeId)?.uploadId === target.uploadId
    );
  }

  #handleFailure(target: AttachmentImportTarget): void {
    if (this.#isCurrent(target)) {
      try {
        target.onFailure(target.item);
      } catch {
        // Call-site reporting must not reject the scheduled worker.
      }
      if (target.removeOnFailure && this.#isCurrent(target)) {
        try {
          target.editor.enqueue({
            type: 'node',
            op: { type: 'delete', id: target.nodeId },
          });
          target.editor.flush();
        } catch {
          // The upload has already failed; cleanup still has to complete.
        }
      }
    }
  }

  #cleanup(target: AttachmentImportTarget, objectUrl?: string): void {
    const pending = this.#pendingMap(target.editor, target.item.kind).get(target.nodeId);
    if (pending?.uploadId === target.uploadId) {
      this.#pendingMap(target.editor, target.item.kind).delete(target.nodeId);
    }
    if (objectUrl !== undefined) URL.revokeObjectURL(objectUrl);
  }

  importAtSelection(
    items: readonly AttachmentImportItem[],
    {
      existingNodeId,
      onFailure,
    }: {
      existingNodeId?: string;
      onFailure: AttachmentImportFailureHandler;
    },
  ): boolean {
    const first = items[0];
    const editor = this.#editableEditor();
    if (!first || !editor) return false;

    const mapped: MappedAttachment[] = [];
    if (existingNodeId !== undefined) {
      if (!this.#isAvailable(editor, existingNodeId, first.kind)) return false;
      mapped.push({ item: first, nodeId: existingNodeId, removeOnFailure: false });
    }
    const tail = existingNodeId === undefined ? items : items.slice(1);

    if (tail.length > 0) {
      const requestId = crypto.randomUUID();
      const nodeIds = this.#collectReceipt(editor, requestId, () => {
        editor.enqueue({
          type: 'insertion',
          op: {
            type: 'attachment_placeholders',
            request_id: requestId,
            kinds: tail.map((item) => item.kind),
          },
        });
      });
      if (!nodeIds || !this.#isExactMapping(nodeIds, tail.length)) return false;
      if (existingNodeId !== undefined && nodeIds.includes(existingNodeId)) {
        console.error('Attachment placeholder receipt contains the existing destination node ID.', {
          existingNodeId,
          nodeIds,
        });
        return false;
      }
      for (const [index, item] of tail.entries()) {
        const nodeId = nodeIds[index];
        if (nodeId === undefined) return false;
        mapped.push({ item, nodeId, removeOnFailure: true });
      }
    }

    return this.#reserveAndSchedule(editor, mapped, onFailure);
  }

  importAtDrop(
    items: readonly AttachmentImportItem[],
    {
      page,
      x,
      y,
      modifiers,
      reuseNodeId,
      onFailure,
    }: {
      page: number;
      x: number;
      y: number;
      modifiers: InputModifiers;
      reuseNodeId?: string;
      onFailure: AttachmentImportFailureHandler;
    },
  ): boolean {
    const first = items[0];
    const editor = this.#editableEditor();
    if (!first || !editor) return false;

    const reuseCandidate = reuseNodeId && this.#isAvailable(editor, reuseNodeId, first.kind) ? reuseNodeId : undefined;
    const requestId = crypto.randomUUID();
    const nodeIds = this.#collectReceipt(editor, requestId, () => {
      editor.enqueue({
        type: 'dnd',
        op: {
          type: 'drop',
          page,
          x,
          y,
          modifiers,
          payload: {
            type: 'files',
            request_id: requestId,
            kinds: items.map((item) => item.kind),
            reuse_node_id: reuseCandidate,
          },
        },
      });
    });
    if (!nodeIds || !this.#isExactMapping(nodeIds, items.length)) return false;
    if (reuseCandidate && nodeIds.indexOf(reuseCandidate) > 0) {
      console.error('Attachment placeholder receipt placed the reuse candidate after the first item.', {
        reuseNodeId: reuseCandidate,
        nodeIds,
      });
      return false;
    }

    const reusedFirst = reuseCandidate !== undefined && nodeIds[0] === reuseCandidate;
    const mapped: MappedAttachment[] = [];
    for (const [index, item] of items.entries()) {
      const nodeId = nodeIds[index];
      if (nodeId === undefined) return false;
      mapped.push({ item, nodeId, removeOnFailure: !reusedFirst || index > 0 });
    }
    return this.#reserveAndSchedule(editor, mapped, onFailure);
  }

  canReusePlaceholder(nodeId: string, kind: AttachmentPlaceholderKind): boolean {
    const editor = this.#ctx.editor;
    return editor !== undefined && this.#isEditorCurrent(editor) && this.#isAvailable(editor, nodeId, kind);
  }

  cancelEditor(editor: Editor): void {
    for (const pending of editor.inflightImages.values()) {
      if (pending.url !== undefined) URL.revokeObjectURL(pending.url);
    }
    editor.inflightImages.clear();
    editor.inflightFiles.clear();
  }

  cancelNode(editor: Editor, nodeId: string): void {
    const image = editor.inflightImages.get(nodeId);
    if (image) {
      editor.inflightImages.delete(nodeId);
      if (image.url !== undefined) URL.revokeObjectURL(image.url);
    }
    editor.inflightFiles.delete(nodeId);
  }
}
