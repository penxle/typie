// cspell:ignore anachronies analepsis focalization heterodiegetic homodiegetic prolepsis

// enum 값의 한국어 풀이 — 산출물 모달의 enum 필에 툴팁으로 붙는다. 값은 원문 그대로 보이고 풀이만 호버로 얹는다.
// 문면은 prism 스키마의 description(값=풀이 쌍)과 STYLISTIC_CRITERIA의 label을 그대로 옮긴 것이다 — 원본:
// prism apps/feedback/stages/{description,rubric,judgment,stylistic}.ts, followup.ts. 같은 값이 필드마다 뜻이 다르므로
// (mixed·undetermined·promoted·withheld) 필드 경로로 키를 잡는다. prism이 값을 늘리면 여기도 함께 늘린다.
const GLOSSES = {
  'movements.mode': {
    scene: '장면적 제시',
    summary: '요약 제시',
    mixed: '혼합',
  },
  'narration.voice.type': {
    homodiegetic: '서술자가 이야기 속 인물',
    heterodiegetic: '서술자가 이야기 밖',
  },
  'narration.situation': {
    'first-person': '1인칭 경험담',
    authorial: '외부 권위 서술',
    figural: '시점 인물의 눈으로 보는 서술',
    mixed: '혼합·전환',
  },
  'narration.overtness.type': {
    overt: '드러난 서술자',
    covert: '숨은 서술자',
  },
  'narration.focalization.type': {
    zero: '무제한',
    internal: '인물 내부 시야',
    external: '외부 관찰만',
  },
  'narration.focalization.pattern': {
    fixed: '시점 인물 하나에 고정',
    variable: '시점 인물이 교대',
    multiple: '같은 사건을 여러 시점 인물이 되풀이 서술',
  },
  'narration.anachronies.kind': {
    analepsis: '회상',
    prolepsis: '예고',
  },
  'narration.anachronies.subjectivity': {
    subjective: '인물의 기억·전망',
    objective: '서술 층위의 사실',
  },
  'narration.discourse.form': {
    'free-indirect': '자유간접화법',
    'interior-monologue': '내적 독백',
    coloring: '문체 물듦',
    other: '기타',
  },
  'experience.kind': {
    reparse: '구문·의미를 재해석함',
    'attribution-uncertain': '화자·지시 대상 확정 불가',
    redundant: '선행 정보의 반복',
    'expectation-shift': '형성된 기대의 전복·충족',
    'info-gap': '이해에 필요한 정보 미제시',
  },
  'audience.source.status': {
    identified: '검색으로 원작 특정',
    'not-identified': '검색했으나 미특정',
    undetermined: '검색 불능',
  },
  'audience.knowledge.source': {
    websearch: '검색으로 확정',
    manuscript: '원고 관측으로 확정',
    author: '작가에게 물어 확정',
    given: '호출자가 준 전제',
  },
  'condition.completeness.level': {
    draft: '초고',
    'in-revision': '퇴고 중',
    complete: '완성고',
    undetermined: '판정 불가',
  },
  'rubric.coverage.from': {
    performance: '등재 수행',
    question: '미결 질문',
  },
  'rubric.coverage.disposition': {
    covered: '특질이 감시',
    dismissed: '감시 불요',
  },
  'judgment.verdicts.basis': {
    carried: '지난 회차 판정을 그대로 잇는다',
    rejudged: '이번 회차에 다시 판정',
  },
  'judgment.log.disposition': {
    promoted: '피드백으로 승격',
    explained: '작품 근거로 해소',
    withheld: '발화 조건 미충족',
  },
  'stylistic.log.disposition': {
    promoted: '문면 피드백으로 승격',
    judged: '판정이 이미 승격',
    withheld: '문면 기준으로도 결함 아님',
  },
  'verification.method': {
    grep: '전문 대조 수행',
    reread: '구간 재열람',
    standalone: '단일 지점 완결',
  },
  'stylistic.criterion': {
    clarity: '명료',
    cohesion: '응집',
    continuity: '연속',
    repetition: '반복',
    language: '언어 일관',
    weight: '무게',
    'manuscript-accident': '원고 사고',
    'narrative-line': '서사 문면',
  },
  'threads.verdict': {
    resolved: '새 원고가 해소했다',
    kept: '여전히 걸린다',
    withdrawn: '작가의 반박이 옳아 물러선다',
  },
} satisfies Record<string, Record<string, string>>;

export type GlossField = keyof typeof GLOSSES;

export const glossOf = (field: GlossField, value: string): string | undefined => (GLOSSES[field] as Record<string, string>)[value];

// 라벨(그룹 제목·필드 키·표 머리글)의 한국어 풀이 — 같은 툴팁 문법으로 호버에 얹는다. 키는 산출물 경로라 같은 이름의
// 키(note·verification…)가 자리마다 다른 뜻을 지녀도 갈린다. 문면은 prism 스키마의 property description을 한 줄로 줄인 것.
const LABEL_GLOSSES = {
  // movements
  'movements.title': '구획의 이름',
  'movements.mode': '제시 방식',
  'movements.basis': '무엇이 이 구획을 한 단위로 만드는가',
  'movements.says': '이 구획이 무엇을 말하는가',
  'movements.does': '이 구획이 독자에게 무엇을 하는가',
  // narration
  'narration.voice': '서술자의 목소리',
  'narration.voice.type': '서술자가 이야기 속 인물인가 밖인가',
  'narration.voice.note': '그렇게 판정한 근거',
  'narration.voice.evidence': '원고에서 글자 그대로 복사한 근거 인용',
  'narration.situation': '서술 상황',
  'narration.overtness': '서술자의 드러남',
  'narration.overtness.type': '드러난 서술자인가 숨은 서술자인가',
  'narration.overtness.note': '관측된 표지 또는 그 부재',
  'narration.focalization': '초점화',
  'narration.focalization.type': '시야의 범위',
  'narration.focalization.pattern': '시점 인물의 운용',
  'narration.focalization.reflectors': '시점 인물로 쓰이는 인물 이름들',
  'narration.tense': '서술 시제와 시간 이탈',
  'narration.tense.base': '기본 서술 시제',
  'narration.anachronies': '시간 이탈 목록',
  'narration.anachronies.kind': '회상인가 예고인가',
  'narration.anachronies.subjectivity': '인물의 기억·전망인가, 서술 층위의 사실인가',
  'narration.anachronies.note': '한 줄 설명',
  'narration.discourse': '담화 재현 기법의 관측 목록',
  'narration.discourse.form': '기법의 종류',
  'narration.discourse.note': '판정 근거',
  'narration.denomination': '인물별 호명 체계',
  'narration.denomination.aliases': '같은 인물을 부르는 다른 이름·애칭들',
  'narration.denomination.note': '호명 관례 한 줄',
  'narration.reliability': '서술 신뢰성의 표지 관측',
  'narration.reliability.note': '관측된 표지',
  // experience
  'experience.kind': '처리 사건의 종류',
  'experience.note': '무슨 일이 일어났는가',
  // audience
  'audience.source': '원작 판정',
  'audience.source.status': '원작을 특정했는가',
  'audience.source.name': '특정된 원작명',
  'audience.source.background': '원작 배경 요약',
  'audience.genre': '이 작품의 상정 독자',
  'audience.knowledge': '상정 독자가 자명하게 아는 것의 목록',
  'audience.knowledge.id': '항목 식별자',
  'audience.knowledge.fact': '독자가 아는 사실 한 줄',
  'audience.knowledge.source': '무엇으로 확정했는가',
  'audience.knowledge.note': '무엇으로 어떻게 확정했는지 한 줄',
  // condition
  'condition.completeness': '원고가 어느 단계에 있는가',
  'condition.completeness.level': '초고 / 퇴고 중 / 완성고 / 판정 불가',
  'condition.completeness.note': '그렇게 본 근거',
  'condition.exclusions': '분석 제외 구간',
  'condition.exclusions.reason': '제외한 이유',
  // interpretation
  'interpretation.hypothesis': '독해 가설',
  'interpretation.hypothesis.statement': '이 작품이 하려는 것',
  'interpretation.hypothesis.effect': '상정 독자에게 일으키려는 효과',
  'interpretation.questions': '해석이 확정하지 못한 것의 명시 목록',
  'interpretation.performances': '작품이 실제로 수행 중인 기법의 등재',
  'interpretation.performances.evidence': '서로 다른 자리의 인용 2개 이상',
  'interpretation.performances.rationale': '이 기법이 무엇이고, 가설의 의도 효과에 어떻게 복무하는가',
  'interpretation.meanings': '인상적으로 작동한 대목의 진술',
  'interpretation.meanings.principle': '어떤 장치가 어떤 배치로 독자에게 무엇을 일으키는가',
  // rubric
  'rubric.traits': '이 작품의 성패를 가르는 특질',
  'rubric.traits.rationale': '독해 가설·작품 성격의 무엇에서 이 특질이 도출되는지와, 실패 시 상정 독자가 치르는 비용',
  'rubric.traits.findings': '발화 조건',
  'rubric.traits.findings.id': '조건 식별자',
  'rubric.traits.findings.condition': '관측 가능한 판별문',
  'rubric.traits.waivers': '면제 조건',
  'rubric.traits.scores': '작품 수준 판정점',
  'rubric.traits.scores.point': '판정점 1~4',
  'rubric.traits.scores.condition': '이 점을 주는 관측 조건',
  'rubric.traits.edges': '엣지 판정 규칙',
  'rubric.traits.verification': '이 특질의 피드백에 요구되는 확인',
  'rubric.coverage': '위험 원천의 전건 처분',
  'rubric.coverage.subject': '수행 id 또는 미결 질문 id',
  'rubric.coverage.from': '수행인가 미결 질문인가',
  'rubric.coverage.disposition': '특질이 감시하는가, 감시 불요인가',
  'rubric.coverage.trait': '감시하는 특질 id',
  'rubric.coverage.note': '배정 근거 또는 기각 사유',
  // judgment
  'judgment.verdicts': '특질마다 하나의 작품 수준 판정',
  'judgment.verdicts.trait': '특질 id',
  'judgment.verdicts.point': '판정점 1~4',
  'judgment.verdicts.note': '그 판정점의 관측 조건과 무엇이 부합·미달했는지',
  'judgment.verdicts.basis': '지난 회차 판정을 이었는가, 다시 판정했는가',
  'judgment.findings': '피드백',
  'judgment.findings.trait': '이 피드백이 속한 특질',
  'judgment.findings.condition': '그 특질의 발화 조건 id',
  'judgment.findings.observation': '발화 조건이 여기서 충족됨의 입증',
  'judgment.findings.verification': '기준표의 확인 요건을 어떻게 이행했는가',
  'judgment.findings.direction': '실행 가능한 제안',
  'judgment.elevations': '한 걸음 더 가 볼 자리',
  'judgment.elevations.trait': '이 격상이 속한 특질',
  'judgment.elevations.observation': '4점 조건의 무엇이 여기서 미달인지와, 그것이 서면 독자가 무엇을 얻는지',
  'judgment.elevations.direction': '작가가 시도할 수 있는 방향',
  'judgment.log': '독서 기록 항목 전건의 처분',
  'judgment.log.entry': '독서 기록 항목의 id',
  'judgment.log.disposition': '승격 / 작품 근거로 해소 / 발화 조건 미충족',
  'judgment.log.finding': '승격됐다면 그 피드백의 id',
  'judgment.log.note': '처분 근거',
  'judgment.gaps': '기준표가 덮지 못한 결함 후보의 신고',
  'judgment.gaps.id': '신고 식별자',
  'judgment.gaps.note': '무엇을 만났고 어느 기준이 비어 있는가',
  threads: '지난 회차 스레드의 처분',
  'threads.thread': '스레드 id',
  'threads.verdict': '해소 / 유지 / 철회',
  'threads.note': '새 원고의 무엇이 그 판정을 지지하는가',
  'threads.anchor': '유지인데 이전 자리가 사라진 경우 새 원고에서 다시 인용한 대목',
  // stylistic
  'stylistic.findings': '문면 피드백',
  'stylistic.findings.criterion': '문면 계열',
  'stylistic.findings.observation': '그 계열의 조건이 여기서 충족됨의 입증',
  'stylistic.findings.verification': '반복·일관 주장을 어떻게 확인했는가',
  'stylistic.findings.direction': '실행 가능한 제안',
  'stylistic.log': '독서 기록 항목 전건의 문면 층위 처분',
  'stylistic.log.entry': '독서 기록 항목의 id',
  'stylistic.log.disposition': '문면 승격 / 판정이 이미 승격 / 문면 기준으로도 결함 아님',
  'stylistic.log.finding': '승격됐다면 그 문면 피드백의 id',
  'stylistic.log.note': '처분 근거',
  'stylistic.coverage': '구획 전건의 커버 기록',
  'stylistic.coverage.movement': '구획 지도의 구획 id',
  'stylistic.coverage.note': '이 구획에서 어느 계열을 대조해 무엇을 확인했는가',
} satisfies Record<string, string>;

export type LabelField = keyof typeof LABEL_GLOSSES;

export const labelGlossOf = (field: LabelField): string => LABEL_GLOSSES[field];
