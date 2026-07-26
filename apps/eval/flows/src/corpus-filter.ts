import { GENRES, normalizeGenre } from './genres.ts';
import type OpenAI from 'openai';

// 코퍼스 후보 심사. "문학적 창작물인가" 하나만 물으면 통과하지만 오라클을 돌릴 수 없는 글이
// 대량으로 들어온다 — v3 30편을 전수조사한 결과 16편이 그랬다. 걸러야 하는 것은 네 가지이며
// 각각을 따로 묻는다. 장르는 층화 표집에 필요하므로 함께 받는다.

export type CorpusSignals = {
  characterCount: number;
  lineCount: number;
  // "치히로:" 처럼 화자 이름만 있는 행. 대본·게임 시나리오의 표지.
  speakerLabelLines: number;
  // "[위치: ...]", "[#3] 정수의 방/ 밤" 같은 장면 지시.
  sceneHeadingLines: number;
  // "01.", "#02_" 로 시작하는 절 머리. 조각글 묶음의 표지.
  numberedSectionLines: number;
  separatorLines: number;
  // 문서 첫머리에 붙은 작가 안내("* ~합니다").
  leadingNoteLines: number;
  urlCount: number;
  // "6장-", "제3화" 처럼 회차를 밝히는 표기.
  episodeMarkers: number;
  // 소제목처럼 홀로 놓인 짧은 행의 실제 문구. 발췌만으로는 문서 중후반의 분기 표지를 놓친다 —
  // 25,721자 문서의 20,608자 지점에 있던 두 번째 엔딩 표지가 세 발췌 구간 어디에도 없었다.
  headings: string[];
};

const SPEAKER_LABEL = /^[가-힣A-Za-z][가-힣A-Za-z0-9 ]{0,12}:$/;
const SCENE_HEADING = /^[[【(].{0,40}[\]】)]$/;
const NUMBERED_SECTION = /^(#?\d{1,2}[._)]|\d{1,2}\s*[.)])\s*\S/;
const SEPARATOR = /^[\s*\-–—=~_·.]{3,}$/;
const LEADING_NOTE = /^[*※]\s*\S/;
const EPISODE_MARKER = /(^|\s)(제?\s?\d{1,3}\s?(장|화|회|부)(?![가-힣])|\d{1,3}장-)/;

// 문장으로 끝나지 않는 짧은 독립 행 — 소제목·절 표지·엔딩 이름이 여기 걸린다.
const isHeading = (line: string) =>
  line.length >= 2 && line.length <= 40 && !/[.!?"”’]$/.test(line) && !/^["“‘']/.test(line) && !/[가-힣]다$/.test(line);

export const corpusSignals = (text: string): CorpusSignals => {
  const lines = text.split('\n');
  const trimmed = lines.map((l) => l.trim()).filter(Boolean);
  // 앞머리 안내는 문서 첫머리에 몰려 있을 때만 의미가 있다.
  const head = trimmed.slice(0, 12);

  return {
    characterCount: [...text].length,
    lineCount: lines.length,
    speakerLabelLines: trimmed.filter((l) => SPEAKER_LABEL.test(l)).length,
    sceneHeadingLines: trimmed.filter((l) => SCENE_HEADING.test(l)).length,
    numberedSectionLines: trimmed.filter((l) => NUMBERED_SECTION.test(l)).length,
    separatorLines: trimmed.filter((l) => SEPARATOR.test(l)).length,
    leadingNoteLines: head.filter((l) => LEADING_NOTE.test(l)).length,
    urlCount: (text.match(/https?:\/\//g) ?? []).length,
    episodeMarkers: trimmed.filter((l) => EPISODE_MARKER.test(l)).length,
    headings: trimmed.filter((l) => isHeading(l)).slice(0, 40),
  };
};

export const describeSignals = (s: CorpusSignals): string =>
  [
    `전체 ${s.characterCount}자, ${s.lineCount}행`,
    s.speakerLabelLines > 0 ? `화자 이름만 있는 행 ${s.speakerLabelLines}개` : null,
    s.sceneHeadingLines > 0 ? `대괄호 장면 지시 ${s.sceneHeadingLines}개` : null,
    s.numberedSectionLines > 0 ? `번호로 시작하는 절 머리 ${s.numberedSectionLines}개` : null,
    s.separatorLines > 0 ? `구분선 ${s.separatorLines}개` : null,
    s.leadingNoteLines > 0 ? `문서 첫머리의 안내 행 ${s.leadingNoteLines}개` : null,
    s.urlCount > 0 ? `바깥 링크 ${s.urlCount}개` : null,
    s.episodeMarkers > 0 ? `회차 표기 ${s.episodeMarkers}개` : null,
  ]
    .filter(Boolean)
    .join(', ') + (s.headings.length > 0 ? `\n소제목처럼 홀로 놓인 행:\n${s.headings.map((h) => `  · ${h}`).join('\n')}` : '');

// 전문을 넘기면 후보 수백 편에 비용이 붙고, 앞부분만 보면 묶음·후기·복수 엔딩을 놓친다.
// 문제는 대개 중간과 끝에서 드러나므로 세 곳을 떠서 함께 보여준다.
export const excerptForClassification = (text: string): string => {
  const chars = [...text];
  if (chars.length <= 7000) return text;
  const head = chars.slice(0, 3000).join('');
  const middleStart = Math.floor(chars.length / 2) - 1000;
  const middle = chars.slice(middleStart, middleStart + 2000).join('');
  const tail = chars.slice(-2000).join('');
  return ['=== 문서 앞부분 ===', head, '', '=== 문서 중간 (일부 생략됨) ===', middle, '', '=== 문서 끝부분 (일부 생략됨) ===', tail].join(
    '\n',
  );
};

export type CorpusVerdict = {
  kind: string;
  genre: string;
  narrative: boolean;
  singleWork: boolean;
  selfContained: boolean;
  original: boolean;
  reason: string;
};

export const isAccepted = (v: Pick<CorpusVerdict, 'narrative' | 'singleWork' | 'selfContained' | 'original'>): boolean =>
  v.narrative && v.singleWork && v.selfContained && v.original;

export const rejectedAxes = (v: Pick<CorpusVerdict, 'narrative' | 'singleWork' | 'selfContained' | 'original'>): string[] =>
  [
    v.narrative ? null : 'narrative',
    v.singleWork ? null : 'singleWork',
    v.selfContained ? null : 'selfContained',
    v.original ? null : 'original',
  ].filter((x): x is string => x !== null);

const SYSTEM_PROMPT = [
  '당신은 문학 피드백 시스템에 넣을 원고를 고르는 심사자입니다.',
  '한 문서가 제공됩니다. 긴 문서는 앞·중간·끝을 발췌해 보여줍니다. 구조 신호도 함께 제공됩니다.',
  '',
  '네 가지를 각각 판정해 classify 도구를 정확히 한 번 호출하세요.',
  '',
  '**narrative — 서사가 전개되는 산문인가**',
  '인물과 사건이 있고 서술로 진행되어야 합니다. 아래는 모두 false입니다.',
  '- 화자 이름을 행마다 붙인 대본·게임 시나리오 형식(예: "치히로:" 다음 줄에 대사)',
  '- 로그라인·시놉시스·플롯 개요·톤앤매너 메모 같은 기획 문서',
  '- 주제를 설명하고 독자에게 교훈이나 당부를 전하는 글',
  '- 세계관 설정, 인물 소개, 연표만 나열한 글',
  '- 일기, 메모, 후기, 리뷰, 공지',
  '시나 짧은 산문시는 서사가 옅어도 true입니다.',
  '',
  '**singleWork — 한 편의 작품인가**',
  '문서 하나에 작품 하나가 담겨야 합니다. 아래는 모두 false입니다.',
  '- "조각글 모음", "단편 모음"처럼 여러 편을 묶었다고 밝힌 글',
  '- 소제목이나 번호로 나뉜 여러 이야기가 서로 이어지지 않는 글',
  '- 같은 이야기의 결말을 여러 갈래로 나란히 실은 글',
  '- 본편 뒤에 설정 메모, 후기, 잡담이 붙은 글',
  '- 남의 시나 가사 전문을 옮겨 붙인 글',
  '장·절로 나뉘어도 하나의 이야기가 이어진다면 true입니다.',
  '',
  '**selfContained — 이 문서만으로 읽히는가**',
  '앞 회차나 선행작을 읽어야 이해되는 글은 false입니다. "전편", "~와 이어지는 글", "6장"처럼',
  '중간부터 시작한다고 밝힌 글이 해당합니다. 연재물이라도 이 문서 안에서 사건이 시작되고',
  '매듭지어진다면 true입니다. 원작이 있는 이차창작에서 원작 세계관을 전제하는 것은 여기 해당하지',
  '않습니다 — 같은 작가의 앞선 회차에 의존하는 경우만 false입니다.',
  '',
  '**original — 옮긴 글이 아닌가**',
  '다른 언어로 쓰인 작품의 번역, 영상·게임 대사의 전사는 false입니다. 원작이 있는 이차창작은',
  '작가가 직접 쓴 것이므로 true입니다. 확신이 없으면 true로 두되 reason에 근거를 적으세요.',
  '',
  `장르는 다음 8종 중 가장 가까운 것을 고르세요: ${GENRES.map((g) => `${g.key}(${g.name})`).join(', ')}. 특정하기 어려우면 'etc'.`,
  '',
  '판단이 갈리면 false로 두세요. 부적합한 원고 하나가 평가자 여러 명의 시간을 버립니다.',
].join('\n');

const classifyTool: OpenAI.Chat.Completions.ChatCompletionFunctionTool = {
  type: 'function',
  function: {
    name: 'classify',
    description: '원고가 문학 피드백 대상으로 적합한지 판정한 결과를 보고합니다.',
    parameters: {
      type: 'object',
      properties: {
        kind: { type: 'string', description: "글의 종류. 예: '단편소설', '조각글 모음', '시나리오 기획', '대본', '에세이'" },
        genre: { type: 'string', enum: GENRES.map((g) => g.key), description: '장르. 8종 중 하나' },
        narrative: { type: 'boolean' },
        singleWork: { type: 'boolean' },
        selfContained: { type: 'boolean' },
        original: { type: 'boolean' },
        reason: { type: 'string', description: 'false로 판정한 항목이 있다면 그 근거를 원문에서 짚어 한두 문장으로' },
      },
      required: ['kind', 'genre', 'narrative', 'singleWork', 'selfContained', 'original', 'reason'],
      additionalProperties: false,
    },
  },
};

export const classifyCorpusDocument = async (openai: OpenAI, model: string, text: string): Promise<CorpusVerdict> => {
  const signals = corpusSignals(text);
  const response = await openai.chat.completions.create({
    model,
    messages: [
      { role: 'system', content: SYSTEM_PROMPT },
      { role: 'user', content: `<구조 신호>\n${describeSignals(signals)}\n</구조 신호>\n\n${excerptForClassification(text)}` },
    ],
    tools: [classifyTool],
    tool_choice: { type: 'function', function: { name: 'classify' } },
  });
  const call = response.choices[0]?.message?.tool_calls?.[0];
  if (!call || call.type !== 'function' || call.function.name !== 'classify') {
    throw new Error('classify tool call missing');
  }
  const parsed = JSON.parse(call.function.arguments) as CorpusVerdict;
  return { ...parsed, genre: normalizeGenre(parsed.genre) };
};
