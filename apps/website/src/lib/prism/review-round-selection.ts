// 여백이 문서별로 저장하는 표시 회차 — 확인 카드가 같은 값을 읽어 기본 계보를 고른다
export const reviewRoundSelectionKey = (documentId: string): string => `typie:prism-review-round:${documentId}`;

export const readReviewRoundSelection = (documentId: string): string | null => {
  try {
    return localStorage.getItem(reviewRoundSelectionKey(documentId));
  } catch {
    return null;
  }
};

export const writeReviewRoundSelection = (documentId: string, roundId: string | null): void => {
  try {
    localStorage.setItem(reviewRoundSelectionKey(documentId), roundId ?? 'none');
  } catch {
    // 저장 실패는 기본값으로 돌아갈 뿐이다
  }
};
