import type { Message } from '@typie/editor-ffi/browser';
import type { FileAsset, FileStage } from '../types';

export const deriveFileStage = ({
  fileId,
  inflight,
  asset,
}: {
  fileId: string | undefined;
  inflight: { name: string; size: number } | undefined;
  asset: FileAsset | undefined;
}): FileStage => {
  if (asset) return 'ready';
  if (inflight) return 'uploading';
  if (fileId != null && fileId !== '') return 'resolving';
  return 'empty';
};

type UploadFileAsFile = (file: File) => Promise<FileAsset>;

export const createSetFileAttrsMessage = (nodeId: string, fileId: string): Message => ({
  type: 'node',
  op: {
    type: 'set_attrs',
    id: nodeId,
    attrs: {
      type: 'file',
      id: fileId,
    },
  },
});

export const createDeleteNodeMessage = (nodeId: string): Message => ({
  type: 'node',
  op: {
    type: 'delete',
    id: nodeId,
  },
});

export const processFileUpload = async ({
  file,
  nodeId,
  setInflightFile,
  deleteInflightFile,
  setFileAsset,
  isCurrent,
  commit,
  focus,
  uploadFileAsFile,
}: {
  file: File;
  nodeId: string;
  setInflightFile: (nodeId: string, inflight: { name: string; size: number }) => void;
  deleteInflightFile: (nodeId: string) => void;
  setFileAsset: (asset: FileAsset) => void;
  isCurrent: () => boolean;
  commit: (message: Message) => void;
  focus: () => void;
  uploadFileAsFile: UploadFileAsFile;
}): Promise<'uploaded' | 'failed' | 'cancelled'> => {
  setInflightFile(nodeId, { name: file.name, size: file.size });

  try {
    const uploaded = await uploadFileAsFile(file);
    if (!isCurrent()) {
      deleteInflightFile(nodeId);
      return 'cancelled';
    }
    setFileAsset(uploaded);
    commit(createSetFileAttrsMessage(nodeId, uploaded.id));
    deleteInflightFile(nodeId);
    focus();
    return 'uploaded';
  } catch {
    deleteInflightFile(nodeId);
    focus();
    return 'failed';
  }
};
