export type MarginJump = { documentId: string; roundId: string; itemId: string | null };

// 대화·총평에서 누른 자리로 여백을 데려간다. 문서를 새로 열어야 하면 그 문서의 여백이 설 때까지 기다려야 하므로
// 요청을 여기 들고 있다가 여백이 가져간다 — 최신 1건만 남긴다.
let pending = $state<MarginJump | null>(null);

export const requestMarginJump = (jump: MarginJump) => {
  pending = jump;
};

export const takeMarginJump = (documentId: string | null): MarginJump | null => {
  const jump = pending;
  if (jump === null || jump.documentId !== documentId) {
    return null;
  }

  pending = null;
  return jump;
};
