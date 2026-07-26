// 개인 열람용 문서 적재의 판정부. 어떤 id를 받아들이고 어떤 id를 왜 거절했는지를 여기서 정한다.

export type PersonalIntakeInput = {
  requestedIds: string[];
  // 이미 적재된 id(refId 기준).
  existingRefIds: string[];
  extracted: { documentId: string; prose: string | null }[];
};

export type PersonalIntakeResult = {
  accepted: { refId: string; prose: string; characterCount: number }[];
  // 이미 있는 글. 문서를 다시 넣지 않고 그대로 쓴다 — 같은 글을 다른 프롬프트 세트로
  // 다시 돌려 견주는 것이 이 기능의 주된 쓰임이라, 중복을 거절하면 길이 막힌다.
  reused: string[];
  rejected: { refId: string; reason: string }[];
};

// 최소 길이 — 프로즈가 사실상 비어 있으면 파이프라인이 읽을 것이 없다.
const MIN_CHARACTERS = 200;

export const parseDocumentIds = (raw: string): string[] => [
  ...new Set(
    raw
      .split(/[\s,]+/)
      .map((s) => s.trim())
      .filter((s) => s.length > 0),
  ),
];

// 공개 여부를 따지지 않는다. 표집 코퍼스는 공개 글만 받지만, 이 경로는 작성자 본인이
// 자기 글의 피드백을 읽으려고 들이는 자리다. 어드민만 쓸 수 있고 어드민은 이미 어느 문서든
// 볼 수 있으므로, 여기서 공개 조건을 요구하는 것은 보호가 아니라 마찰이다.
export const planPersonalIntake = (input: PersonalIntakeInput): PersonalIntakeResult => {
  const existingSet = new Set(input.existingRefIds);
  const proseById = new Map(input.extracted.map((e) => [e.documentId, e.prose]));

  const accepted: PersonalIntakeResult['accepted'] = [];
  const reused: string[] = [];
  const rejected: PersonalIntakeResult['rejected'] = [];

  for (const refId of input.requestedIds) {
    if (existingSet.has(refId)) {
      reused.push(refId);
      continue;
    }
    const prose = proseById.get(refId);
    if (!prose) {
      rejected.push({ refId, reason: '없는 문서이거나 본문을 추출하지 못했습니다' });
      continue;
    }
    const characterCount = [...prose].length;
    if (characterCount < MIN_CHARACTERS) {
      rejected.push({ refId, reason: `본문이 너무 짧습니다 (${characterCount}자)` });
      continue;
    }
    accepted.push({ refId, prose, characterCount });
  }

  return { accepted, reused, rejected };
};
