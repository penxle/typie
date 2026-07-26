export type ContributionJudgment = { id: string; taskId: string; evaluatorEmail: string; createdAt: Date };

// 유효 기여 — 태스크별 필요 수 안에 든(생성 시간순 선착) 판정만 센다. 잉여 판정을 캡에 계상하면
// 라운드 총 용량(캡 × 인원)이 필요 총합보다 먼저 소진되어 뒤쪽 문서가 평가를 받지 못한다.
// requiredJudgments가 null인 태스크(중복 구간)는 상한이 없어 참여자 전원의 판정이 유효하다 —
// 1로 세면 두 번째 사람부터의 판정이 통째로 잉여 취급되어 기여가 사라진다.
export const effectiveContributions = (
  tasks: { id: string; requiredJudgments: number | null }[],
  judgments: ContributionJudgment[],
  participants: number,
): Map<string, number> => {
  const open = Math.max(1, participants);
  const requiredByTask = new Map(tasks.map((t) => [t.id, t.requiredJudgments ?? open]));
  const byTask = new Map<string, ContributionJudgment[]>();
  for (const judgment of judgments) {
    const list = byTask.get(judgment.taskId) ?? [];
    list.push(judgment);
    byTask.set(judgment.taskId, list);
  }

  const counts = new Map<string, number>();
  for (const [taskId, list] of byTask) {
    const required = requiredByTask.get(taskId) ?? 1;
    const sorted = list.toSorted((a, b) => a.createdAt.getTime() - b.createdAt.getTime() || a.id.localeCompare(b.id));
    for (const judgment of sorted.slice(0, required)) {
      counts.set(judgment.evaluatorEmail, (counts.get(judgment.evaluatorEmail) ?? 0) + 1);
    }
  }
  return counts;
};
