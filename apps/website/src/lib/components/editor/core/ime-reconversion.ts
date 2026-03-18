export type ReconversionCandidate = {
  text: string;
  commitKeyEventSerial: number;
  reopenDeleteKeyEventSerial: number | null;
};

export type BlockedRecomposition = {
  inputValue: string;
  deleteKeyEventSerial: number | null;
  suppressing: boolean;
  shortcutPrimed: boolean;
};

export type ReconversionState = {
  candidate: ReconversionCandidate | null;
  blocked: BlockedRecomposition | null;
  replacementSourceText: string | null;
};

export const createReconversionState = (): ReconversionState => ({
  candidate: null,
  blocked: null,
  replacementSourceText: null,
});

export const countTextChars = (text: string) => [...text].length;

export const getCommonSuffix = (left: string, right: string) => {
  const leftChars = [...left];
  const rightChars = [...right];
  let suffixLength = 0;

  while (
    suffixLength < leftChars.length &&
    suffixLength < rightChars.length &&
    leftChars[leftChars.length - 1 - suffixLength] === rightChars[rightChars.length - 1 - suffixLength]
  ) {
    suffixLength += 1;
  }

  return suffixLength === 0 ? '' : rightChars.slice(rightChars.length - suffixLength).join('');
};

export const clearReconversionCandidate = (reconversion: ReconversionState) => {
  if (!reconversion.candidate) return;
  reconversion.candidate = null;
};

export const setReconversionCandidate = (reconversion: ReconversionState, text: string, commitKeyEventSerial: number) => {
  reconversion.candidate =
    text.length > 0
      ? {
          text,
          commitKeyEventSerial,
          reopenDeleteKeyEventSerial: null,
        }
      : null;
};

export const clearBlockedRecomposition = (reconversion: ReconversionState) => {
  if (!reconversion.blocked) return;
  reconversion.blocked = null;
};

export const clearReplacementSourceText = (reconversion: ReconversionState) => {
  if (reconversion.replacementSourceText === null) return;
  reconversion.replacementSourceText = null;
};

export const clearReconversionState = (reconversion: ReconversionState) => {
  clearReconversionCandidate(reconversion);
  clearBlockedRecomposition(reconversion);
  clearReplacementSourceText(reconversion);
};

export const keepCurrentCandidateForKeyEvent = (reconversion: ReconversionState, keyEventSerial: number) => {
  if (!reconversion.candidate) return false;

  return keyEventSerial === reconversion.candidate.reopenDeleteKeyEventSerial;
};

export const blockedRecompositionIsActive = (reconversion: ReconversionState, keyEventSerial: number) =>
  reconversion.blocked !== null &&
  (reconversion.blocked.suppressing || reconversion.blocked.deleteKeyEventSerial === keyEventSerial || reconversion.blocked.shortcutPrimed);

export const suppressBlockedRecomposition = (reconversion: ReconversionState, keyEventSerial: number) => {
  if (!blockedRecompositionIsActive(reconversion, keyEventSerial)) return false;
  if (reconversion.blocked) {
    reconversion.blocked.suppressing = true;
  }
  return true;
};

export const markBlockedRecompositionDelete = (reconversion: ReconversionState, keyEventSerial: number, suppress = false) => {
  if (!reconversion.blocked) return;
  reconversion.blocked.deleteKeyEventSerial = keyEventSerial;
  if (suppress) {
    reconversion.blocked.suppressing = true;
  }
};

export const startBlockedRecomposition = (reconversion: ReconversionState, value: string) => {
  reconversion.blocked = {
    inputValue: value,
    deleteKeyEventSerial: null,
    suppressing: false,
    shortcutPrimed: false,
  };
  clearReconversionCandidate(reconversion);
};
