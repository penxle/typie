import type Anthropic from '@anthropic-ai/sdk';

// 재설계 파이프라인의 도구 정의. 단계마다 구조화 출력을 tool call로 강제한다.
//
// strict를 켜면 모델이 스키마를 어긴 입력을 낼 수 없다. 실측에서 짚을 곳 찾기의 모든 호출이
// 첫 시도에 스키마를 어겨 재시도했고, 그 재시도가 그 단계 입력을 2.4배로 불렸다.
// strict의 조건은 모든 객체에 additionalProperties: false와 required가 있는 것이다.
const strict = <T extends { name: string }>(tool: T) => ({ ...tool, strict: true as const });

export const SURVEY_TOOL: Anthropic.Messages.Tool = strict({
  name: 'report_profile',
  description: '원고를 읽고 파악한 사실을 전달한다.',
  input_schema: {
    type: 'object',
    properties: {
      form: { type: 'string', description: '글의 형식과 그렇게 본 근거' },
      // 최종 판정은 검색(BACKGROUND)이 내린다. 여기 값은 검색어 힌트일 뿐이다.
      isDerivative: { type: 'boolean', description: '실존 작품의 인물·설정로 짐작되면 참. 확신 불요 — 최종 판정은 시스템이 검색으로 한다' },
      derivativeSource: { type: 'string', description: '짐작되는 원작 후보명. 확신이 없어도 적는다. 모르겠으면 빈 문자열' },
      pov: { type: 'string', description: '작품 전체를 관통하는 시점. 중간에 변하면 변화 양상까지' },
      reliability: { type: 'string', description: '화자 신뢰성과 판단 근거' },
      tense: { type: 'string' },
      dialogueConvention: { type: 'string' },
      deliberateStyles: {
        type: 'array',
        description: '두 곳 이상에서 반복 확인된 의도적 문체만',
        items: {
          type: 'object',
          properties: { pattern: { type: 'string' }, evidence: { type: 'string' } },
          required: ['pattern', 'evidence'],
          additionalProperties: false,
        },
      },
      properNouns: {
        type: 'array',
        description: '인물 이름과 설정 용어. 같은 인물의 다른 호칭·애칭은 함께 묶어서',
        items: { type: 'string' },
      },
      nonAnalyticRanges: {
        type: 'array',
        items: {
          type: 'object',
          properties: { startQuote: { type: 'string' }, endQuote: { type: 'string' }, reason: { type: 'string' } },
          required: ['startQuote', 'endQuote', 'reason'],
          additionalProperties: false,
        },
      },
      scenes: {
        type: 'array',
        description: '원고 전체를 빠짐없이 덮어야 한다',
        items: {
          type: 'object',
          properties: {
            startQuote: { type: 'string', description: '장면 시작 문구를 원고에서 그대로 인용 (20자 안팎)' },
            endQuote: { type: 'string', description: '장면 끝 문구를 원고에서 그대로 인용 (20자 안팎)' },
            gist: { type: 'string' },
            characters: { type: 'array', items: { type: 'string' } },
            setting: { type: 'string' },
            pov: { type: 'string' },
            flashback: { type: 'string', description: '회상·삽입 장면이면 그 사실과 현재 복귀 지점. 아니면 빈 문자열' },
            boundaryQuality: { type: 'string', enum: ['clean', 'weak', 'none'] },
          },
          required: ['startQuote', 'endQuote', 'gist', 'characters', 'setting', 'pov', 'flashback', 'boundaryQuality'],
          additionalProperties: false,
        },
      },
    },
    required: [
      'form',
      'isDerivative',
      'derivativeSource',
      'pov',
      'reliability',
      'tense',
      'dialogueConvention',
      'deliberateStyles',
      'properNouns',
      'nonAnalyticRanges',
      'scenes',
    ],
    additionalProperties: false,
  },
});

// 축 수(3~6)·보호 상한(8)은 여기 적지 않고 plan-check가 강제한다 — strict의 제약 디코딩이
// 배열 길이 키워드를 지원하는지 확인되지 않았고, 400으로 죽느니 코드에서 반려·재시도한다.
export const PLAN_TOOL: Anthropic.Messages.Tool = strict({
  name: 'report_plan',
  description: '이 글에 대한 비평 계획을 전달한다.',
  input_schema: {
    type: 'object',
    properties: {
      intent: { type: 'string', description: '이 글이 하려는 것' },
      protected: {
        type: 'array',
        description: '검토가 건드리면 안 되는 핵심 기법. 본문이 실제로 수행하는 것만. 최대 8개',
        items: {
          type: 'object',
          properties: {
            technique: { type: 'string' },
            evidence: {
              type: 'array',
              description: '원고에서 글자 그대로 복사한 인용. 각 항목은 연속된 한 구절',
              items: { type: 'string' },
            },
            rationale: { type: 'string', description: '이 기법이 무엇을 위해 기능하는가' },
          },
          required: ['technique', 'evidence', 'rationale'],
          additionalProperties: false,
        },
      },
      rejectedFindings: {
        type: 'array',
        description: '반영하지 않은 검수 발견과 사유. 침묵 기각 금지. 초안에서는 빈 배열',
        items: {
          type: 'object',
          properties: {
            target: { type: 'string' },
            reason: { type: 'string' },
          },
          required: ['target', 'reason'],
          additionalProperties: false,
        },
      },
      axes: {
        type: 'array',
        description: '이 글에서 실제로 실패 위험이 있는 검토 축. 정확히 3~6개',
        items: {
          type: 'object',
          properties: {
            label: { type: 'string', description: '짧은 명사구 이름 (20자 이내)' },
            description: { type: 'string', description: '검토자가 무엇을 어떻게 확인해야 하는지. 질문형 지시, 결론 금지' },
            risk: { type: 'string', description: '왜 이 글에서 실제로 위험한가' },
            evidence: {
              type: 'array',
              description: '위험 신호가 나타난 자리의 원고 인용. 글자 그대로, 연속된 구절',
              items: { type: 'string' },
            },
          },
          required: ['label', 'description', 'risk', 'evidence'],
          additionalProperties: false,
        },
      },
    },
    required: ['intent', 'protected', 'rejectedFindings', 'axes'],
    additionalProperties: false,
  },
});

// stumbleQuote가 이 스키마의 핵심이다.
//
// 앞선 판(v1.7)은 주장의 종류를 claimType으로 가르고 종류에 따라 근거 인용을 요구했다. 실측에서
// 그 요구가 한 번도 발동하지 않았다 — 문서 하나가 omission을 0건 내면서, 정작 산출된 지적 열 중
// 넷이 작가에게 무언가를 추가하라고 요구했다. local이 근거 의무가 없어 더 쌌기 때문이다.
//
// 조건부 필드는 언제나 그렇게 된다. 요구가 모델이 고른 구분자에 걸려 있으면 모델은 싼 쪽을
// 고르고, 구분자를 늘리면 도피처도 함께 는다. 그래서 구분자를 없애고 요구를 무조건으로 만든다.
//
// 요구하는 것은 이 단계가 이미 선언한 기준 그대로다 — "읽다가 실제로 멈춘 곳만 적으세요."
// 멈춘 곳을 원문에서 인용하게 하면, 그 기준을 충족한다는 주장이 말이 아니라 좌표가 된다.
// 코드는 그 인용이 원고에 있는지와 분석 대상 안인지를 대조한다.
export const REVIEW_TOOL: Anthropic.Messages.Tool = strict({
  name: 'report_findings',
  description: '읽으면서 걸린 지점들과 잘 작동하는 대목을 전달한다.',
  input_schema: {
    type: 'object',
    properties: {
      findings: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            quoteStart: { type: 'string', description: '대목의 시작 문구를 원고에서 그대로 인용 (15자 안팎)' },
            quoteEnd: { type: 'string', description: '끝 문구를 원고에서 그대로 인용 (15자 안팎)' },
            kind: { type: 'string', enum: ['error', 'readability', 'structure'] },
            stumbleQuote: {
              type: 'string',
              description: '읽다가 실제로 멈춘 바로 그 자리를 원고에서 그대로 인용 (한 구절이면 충분)',
            },
            intent: { type: 'string', description: '이 대목이 하려는 일을 작가의 편에서 읽어낸 것' },
            observation: { type: 'string', description: '읽으면서 무엇이 걸렸는가' },
            cause: { type: 'string', description: '본문의 어떤 구조가 그렇게 만드는가' },
            direction: { type: 'string', description: '30분 안에 시도할 수 있을 만큼 구체적으로. 문장을 대신 써주지 말 것' },
            evidence: { type: 'string' },
          },
          required: ['quoteStart', 'quoteEnd', 'kind', 'stumbleQuote', 'intent', 'observation', 'cause', 'direction', 'evidence'],
          additionalProperties: false,
        },
      },
      // 강점은 짚을 곳과 같은 모양을 하지 않는다. direction을 요구했더니 강점 120건 중 59건이
      // "그대로 두세요"로 끝났고, 평가자들은 그걸 과도한 참견으로 읽었다. 여기서는 무엇이 왜
      // 작동하는지까지만 받고, 이 값은 중복 묶기·피드백 쓰기를 거치지 않고 총평으로 바로 간다.
      strengths: {
        type: 'array',
        description: '잘 작동하는 대목. 작가가 손댈 것이 없으므로 조언을 붙이지 않는다',
        items: {
          type: 'object',
          properties: {
            quoteStart: { type: 'string', description: '대목의 시작 문구를 원고에서 그대로 인용 (15자 안팎)' },
            quoteEnd: { type: 'string', description: '끝 문구를 원고에서 그대로 인용 (15자 안팎)' },
            principle: { type: 'string', description: '무엇이 왜 작동하는지. 조언이나 확장 제안을 붙이지 말 것' },
          },
          required: ['quoteStart', 'quoteEnd', 'principle'],
          additionalProperties: false,
        },
      },
    },
    required: ['findings', 'strengths'],
    additionalProperties: false,
  },
});

// 계획이 예견할 수 없는 원고 사고(작업 메모·편집 잔여물)의 고정 축. 축 enum에 항상 포함된다.
export const ACCIDENT_AXIS = '원고 사고';

// 축 enum이 문서마다 다르므로 도구를 문서 단위로 짓는다. strict가 요청마다 스키마를
// 문법으로 컴파일하니 동적 enum도 그대로 강제된다.
export const reviewToolWithAxes = (axes: string[]): Anthropic.Messages.Tool => {
  if (axes.length === 0) throw new Error('reviewToolWithAxes: 축이 비어 있다');
  const schema = REVIEW_TOOL.input_schema as {
    properties: { findings: { items: { properties: Record<string, unknown>; required: string[] } } } & Record<string, unknown>;
  } & Record<string, unknown>;
  const items = schema.properties.findings.items;
  return strict({
    ...REVIEW_TOOL,
    input_schema: {
      ...schema,
      properties: {
        ...schema.properties,
        findings: {
          type: 'array',
          items: {
            ...items,
            properties: { ...items.properties, axis: { type: 'string', enum: [...axes, ACCIDENT_AXIS] } },
            required: [...items.required, 'axis'],
          },
        },
      },
    } as Anthropic.Messages.Tool['input_schema'],
  });
};

export const DEDUPE_TOOL: Anthropic.Messages.Tool = strict({
  name: 'report_groups',
  description: '같은 문제를 말하는 지적끼리 묶어 전달한다.',
  input_schema: {
    type: 'object',
    properties: {
      groups: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            members: { type: 'array', items: { type: 'integer' } },
            representative: { type: 'integer' },
            reason: { type: 'string' },
          },
          required: ['members', 'representative', 'reason'],
          additionalProperties: false,
        },
      },
    },
    required: ['groups'],
    additionalProperties: false,
  },
});

// 지적 여러 건을 한 호출에서 판정한다. 같은 원문을 여러 번 보내지 않으려는 것이 목적이며,
// 판정 자체는 지적마다 독립이어야 한다 — 서로 견주게 두면 상대적 중요도가 섞인다.
// 그래서 출력도 지적별로 분리된 객체를 요구한다.
export const VERIFY_TOOL: Anthropic.Messages.Tool = strict({
  name: 'report_verdicts',
  description: '지적 각각의 타당성을 서로 견주지 말고 하나씩 판정한다.',
  input_schema: {
    type: 'object',
    properties: {
      findings: {
        type: 'array',
        description: '지적마다 하나씩. 주어진 지적 수와 같아야 한다',
        items: {
          type: 'object',
          properties: {
            index: { type: 'integer', description: '판정 대상 지적의 번호' },
            anchors: {
              type: 'array',
              description: '그 지적의 위치마다 하나씩. 주어진 위치 수와 같아야 한다',
              items: {
                type: 'object',
                properties: {
                  index: { type: 'integer' },
                  ground: { type: 'string', enum: ['valid', 'evidence-missing', 'deliberate-style', 'out-of-scope'] },
                  reason: { type: 'string' },
                },
                required: ['index', 'ground', 'reason'],
                additionalProperties: false,
              },
            },
          },
          required: ['index', 'anchors'],
          additionalProperties: false,
        },
      },
    },
    required: ['findings'],
    additionalProperties: false,
  },
});

// polarity가 없다. 여기 오는 것은 전부 짚을 곳이며 강점은 이 경로를 타지 않는다 —
// 두 층위를 한 스트림에 섞어 두었더니 같은 구간을 지적하고 곧바로 칭찬하는 쌍이 편당 5.2개
// 나왔고, 평가자 셋이 따로 그것을 혼란으로 지목했다.
export const COMPOSE_TOOL: Anthropic.Messages.Tool = strict({
  name: 'report_feedbacks',
  description: '작가가 읽을 피드백 목록을 전달한다.',
  input_schema: {
    type: 'object',
    properties: {
      feedbacks: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            groupIndex: { type: 'integer' },
            category: { type: 'string', description: '2~10자 한국어' },
            body: { type: 'string' },
          },
          required: ['groupIndex', 'category', 'body'],
          additionalProperties: false,
        },
      },
    },
    required: ['feedbacks'],
    additionalProperties: false,
  },
});

// 완성된 피드백 하나가 원고에 근거를 두는지만 본다. 한 번에 하나씩 묻는 것이 핵심이다 —
// 여러 건을 묶어 물으면 판정이 죽는다(배치 8에서 기각 0건, 피드백 쓰기에 섞었을 때도 0건).
export const SELFCHECK_TOOL: Anthropic.Messages.Tool = strict({
  name: 'report_grounding',
  description: '피드백이 원고에 근거를 두는지 판정한다.',
  input_schema: {
    type: 'object',
    properties: {
      grounded: { type: 'boolean', description: '원고에서 근거가 확인되면 true' },
      reason: { type: 'string', description: '원고의 무엇을 확인했는지. 거짓이면 무엇이 어긋났는지' },
    },
    required: ['grounded', 'reason'],
    additionalProperties: false,
  },
});

// 필드 순서가 곧 생성 순서다 — 판정(verdict)을 항변(defense) 뒤에 두어, 기각할 이유를
// 찾아보기 전에는 판정을 서술할 수 없게 한다.
export const DEFENSE_TOOL: Anthropic.Messages.Tool = strict({
  name: 'report_defense',
  description: '지적에 대한 저자의 최선의 항변을 세우고, 그 항변이 원고에서 성립하는지 판정한다.',
  input_schema: {
    type: 'object',
    properties: {
      defense: { type: 'string', description: '저자로서 쓴 최선의 항변 — 나는 ~를 위해 의도적으로 그렇게 썼다' },
      assessment: { type: 'string', description: '항변이 원고에서 성립하는지. 본문의 무엇이 항변을 지지하거나 꺾는지' },
      verdict: {
        type: 'string',
        enum: ['dismiss', 'uphold'],
        description: 'dismiss = 항변 성립, 지적을 내보내지 않는다 / uphold = 항변 불성립, 지적 유지',
      },
    },
    required: ['defense', 'assessment', 'verdict'],
    additionalProperties: false,
  },
});

export const COMPOSE_REVIEW_TOOL: Anthropic.Messages.Tool = strict({
  name: 'report_review',
  description: '작품 전체에 대한 판단을 전달한다.',
  input_schema: {
    type: 'object',
    properties: {
      characterization: { type: 'string' },
      // 강점이 인라인에서 빠진 대신 여기가 그것을 다루는 유일한 자리다. 작품 전체에 대한
      // 판정이므로 목록으로 받되, 어느 대목인지는 앵커로 남겨 화면이 되찾을 수 있게 한다.
      strengths: {
        type: 'array',
        description: '최대 5개. 주어진 강점 후보에서 고르고 새로 만들지 말 것',
        items: {
          type: 'object',
          properties: {
            body: { type: 'string', description: '무엇이 어떻게 작동하는지' },
            quoteStart: { type: 'string', description: '후보로 주어진 인용을 그대로 옮길 것' },
            quoteEnd: { type: 'string', description: '후보로 주어진 인용을 그대로 옮길 것' },
          },
          required: ['body', 'quoteStart', 'quoteEnd'],
          additionalProperties: false,
        },
      },
      patterns: {
        type: 'array',
        description: '최대 5개',
        items: {
          type: 'object',
          properties: {
            theme: { type: 'string' },
            body: { type: 'string', description: '피드백 번호를 쓰지 말 것' },
            feedbackIndexes: { type: 'array', items: { type: 'integer' } },
          },
          required: ['theme', 'body', 'feedbackIndexes'],
          additionalProperties: false,
        },
      },
      priority: {
        type: 'array',
        description: '순서대로',
        items: {
          type: 'object',
          properties: {
            body: { type: 'string', description: '피드백 번호를 쓰지 말 것' },
            feedbackIndexes: { type: 'array', items: { type: 'integer' } },
          },
          required: ['body', 'feedbackIndexes'],
          additionalProperties: false,
        },
      },
    },
    required: ['characterization', 'strengths', 'patterns', 'priority'],
    additionalProperties: false,
  },
});

// 원작 배경 요약. 검색 결과를 그대로 흘리지 않고 "독자가 전제하고 읽는 것"만 추리게 한다.
export const BACKGROUND_TOOL: Anthropic.Messages.Tool = strict({
  name: 'submit_background',
  description: '원작을 모르는 사람이 이 글을 읽을 때 놓치는 전제를 정리한다.',
  // 2차 창작 판정의 권위는 검색이다 — 모델의 단일 판단은 같은 문서에서도 흔들린다(실측:
  // 특정 2차창작 문서가 실행에 따라 아니오로 판정돼 배경이 통째로 빠졌다). 검색 결과에서 원작이
  // 특정되는지가 판정이며, 세 필드 모두 무조건 채운다.
  input_schema: {
    type: 'object',
    properties: {
      sourceIdentified: { type: 'boolean', description: '검색 결과에서 원작이 특정되면 참' },
      sourceName: { type: 'string', description: '특정된 원작명. 특정하지 못했으면 빈 문자열' },
      brief: { type: 'string', description: '원작 배경 브리프. 특정하지 못했으면 빈 문자열' },
      genreVariant: { type: 'string', description: '장르 변형(세계관 AU)이 시사되면 통용 명칭(예: 센티넬버스). 아니면 빈 문자열' },
    },
    required: ['sourceIdentified', 'sourceName', 'brief', 'genreVariant'],
    additionalProperties: false,
  },
});

// 장르 변형(세계관 AU)의 문법 정리. 검토·검수가 장르 독자에게 자명한 것을 결함으로
// 읽지 않게 하는 두 번째 검색의 산출이다.
export const GENRE_TOOL: Anthropic.Messages.Tool = strict({
  name: 'submit_genre',
  description: '장르 변형의 관습 정리를 전달한다.',
  input_schema: {
    type: 'object',
    properties: {
      conventions: { type: 'string', description: '장르 독자가 전제하는 문법. 검색 결과가 지지하지 않으면 빈 문자열' },
    },
    required: ['conventions'],
    additionalProperties: false,
  },
});
