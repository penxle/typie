// 총평이 지적을 가리키는 참조 — 지적을 번호로 세는 티어는 그 회차 result.issues의 인덱스를, id로 다루는 티어는
// 이슈의 id를 싣는다(types.ts FeedbackConclusion). 한 회차의 배열은 한 형태로만 오지만, 저장된 구 결과가
// 영구히 번호라 화면은 양쪽을 상시 받는다.
export type IssueRef = number | string;

// 참조가 겨눌 수 있는 스레드의 최소 형태 — 세 화면(칩·패널·카드 콜아웃)이 이 축만 공유한다.
type IssueRefThread = { id: string; issueIndex: number; issueId: string | null };

// 번호는 그 회차 번호 공간(issueIndex)과 대조하고, id는 이슈 신원(issueId)으로 찾는다. id로 못 찾으면 스레드
// id로 한 번 더 찾는다 — 승계 스레드의 issueId는 탄생 회차의 것이라(사영은 갱신하지 않는다) 지난 회차 지적을
// 가리키는 참조가 스레드 id로 서고, id 도입 전에 굳은 스레드는 issueId가 영구 null이다.
export const threadOfIssueRef = <T extends IssueRefThread>(ref: IssueRef, threads: T[]): T | null => {
  if (typeof ref === 'number') return threads.find((thread) => thread.issueIndex === ref) ?? null;
  return threads.find((thread) => thread.issueId === ref) ?? threads.find((thread) => thread.id === ref) ?? null;
};

// 참조 목록을 스레드 줄로 편다 — 순서는 참조 순서다. 가리키는 스레드가 없는 참조는 접고, 같은 스레드를 두 번
// 가리키면 한 번만 세운다(모델 산출물이라 중복을 가정한다 — each 키가 겹치면 터진다).
export const threadsOfIssueRefs = <T extends IssueRefThread>(issues: readonly IssueRef[], threads: T[]): T[] => {
  const seen = new Set<string>();
  return issues.flatMap((ref) => {
    const thread = threadOfIssueRef(ref, threads);
    if (thread === null || seen.has(thread.id)) return [];
    seen.add(thread.id);
    return [thread];
  });
};

// 이 참조 목록이 스레드를 가리키는가 — 활성 카드 연동(펼침·명도 후퇴)과 카드 콜아웃의 대조축이다. 번호 참조는
// 회차마다 0부터 다시 세므로 남의 회차 스레드에도 걸린다 — 회차 판별은 호출처가 이미 가른다(표시 회차 스레드만
// 넘기거나 ofThisRound로 거른다).
export const issueRefsInclude = (issues: readonly IssueRef[], thread: IssueRefThread | null): boolean =>
  thread !== null &&
  issues.some((ref) => (typeof ref === 'number' ? ref === thread.issueIndex : ref === thread.issueId || ref === thread.id));
