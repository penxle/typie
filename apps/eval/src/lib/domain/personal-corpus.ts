// 개인 열람용 문서 적재의 판정부. 어떤 id를 받아들이고 어떤 id를 왜 거절했는지를 여기서 정한다.

export type PersonalIntakeInput = {
  requestedIds: string[];
  // 공개 조건을 통과해 돌아온 id. 표집 경로와 같은 관문을 쓴다.
  publicIds: string[];
  // 이미 적재된 id(refId 기준). 같은 글을 두 번 들이면 열람 링크가 갈린다.
  existingRefIds: string[];
  extracted: { documentId: string; prose: string | null }[];
};

export type PersonalIntakeResult = {
  accepted: { refId: string; prose: string; characterCount: number }[];
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

export const planPersonalIntake = (input: PersonalIntakeInput): PersonalIntakeResult => {
  const publicSet = new Set(input.publicIds);
  const existingSet = new Set(input.existingRefIds);
  const proseById = new Map(input.extracted.map((e) => [e.documentId, e.prose]));

  const accepted: PersonalIntakeResult['accepted'] = [];
  const rejected: PersonalIntakeResult['rejected'] = [];

  for (const refId of input.requestedIds) {
    if (existingSet.has(refId)) {
      rejected.push({ refId, reason: '이미 들여온 글입니다' });
      continue;
    }
    if (!publicSet.has(refId)) {
      rejected.push({ refId, reason: '공개 상태가 아니거나 존재하지 않는 글입니다' });
      continue;
    }
    const prose = proseById.get(refId);
    if (!prose) {
      rejected.push({ refId, reason: '본문을 추출하지 못했습니다' });
      continue;
    }
    const characterCount = [...prose].length;
    if (characterCount < MIN_CHARACTERS) {
      rejected.push({ refId, reason: `본문이 너무 짧습니다 (${characterCount}자)` });
      continue;
    }
    accepted.push({ refId, prose, characterCount });
  }

  return { accepted, rejected };
};
