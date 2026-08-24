// 리뷰 모달에서 그 리뷰를 진행한 대화로 데려간다. 패널의 세션 선택은 패널 내부 상태라 밖에서 못 만지므로
// 요청을 여기 들고 있다가 패널이 가져간다 — 최신 1건만 남긴다.
let pending = $state<string | null>(null);

export const requestSessionJump = (sessionId: string) => {
  pending = sessionId;
};

export const takeSessionJump = (): string | null => {
  const sessionId = pending;
  if (sessionId === null) {
    return null;
  }

  pending = null;
  return sessionId;
};
