export const getClipboardImageFiles = (clipboardData: DataTransfer | null): File[] => {
  if (!clipboardData) {
    return [];
  }

  const fromFiles = [...clipboardData.files].filter((file) => file.type.startsWith('image/'));
  if (fromFiles.length > 0) {
    return fromFiles;
  }

  const fromItems: File[] = [];
  for (const item of clipboardData.items) {
    if (!item.type.startsWith('image/')) continue;
    const file = item.getAsFile();
    if (file) {
      fromItems.push(file);
    }
  }
  return fromItems;
};

export const getDataTransferImageFiles = (dataTransfer: DataTransfer | null): File[] => {
  if (!dataTransfer) {
    return [];
  }
  return [...dataTransfer.files].filter((file) => file.type.startsWith('image/'));
};

type PendingImageCandidate = {
  nodeId: string;
  imageId?: string;
  assigned: boolean;
  inflight: boolean;
};

export const assignPendingImageFiles = (
  candidates: PendingImageCandidate[],
  pendingFiles: File[],
): { assignments: { nodeId: string; file: File }[]; remainingFiles: File[] } => {
  const remainingFiles = [...pendingFiles];
  const assignments: { nodeId: string; file: File }[] = [];

  for (const candidate of candidates) {
    if (remainingFiles.length === 0) {
      break;
    }
    if (candidate.imageId || candidate.assigned || candidate.inflight) {
      continue;
    }

    const file = remainingFiles.shift();
    if (!file) {
      break;
    }
    assignments.push({ nodeId: candidate.nodeId, file });
  }

  return { assignments, remainingFiles };
};
