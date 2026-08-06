// Editorial 파이프라인의 계약 — 관찰 도구(모델이 직접 호출)와 산출물 스키마(초안 YAML의
// 검증 원천이자 작성 안내의 렌더 원천). 스펙 §4~§8을 wire로 옮긴 것.
// 배열 길이 방지선(축 개수 등)은 스키마에 적지 않고 editorial-checks가 코드로 강제한다.
// 필드 description은 모델에게 그대로 렌더된다(deliverableGuide) — 작성 지침 문체를 유지할 것.
import type Anthropic from '@anthropic-ai/sdk';

const strict = <T extends { name: string }>(tool: T) => ({ ...tool, strict: true as const });

// 문면 층위는 국소 스테이지가 단독 소유한다 — EXECUTE는 계획 축만 받는다.
export const ACCIDENT_AXIS = '원고 사고';
export const SENTENCE_AXIS = '문장 결';
export const LOCAL_AXES = [SENTENCE_AXIS, ACCIDENT_AXIS];

// read/grep/write/edit는 워크스페이스(core/worker/workspace.ts)의 fileTools가 소유한다 —
// 여기는 검색(비결정적, 캐시 대상)만 남는다.
export const SEARCH_TOOL: Anthropic.Messages.Tool = strict({
  name: 'search',
  description: '웹 검색. 원작·장르 문법·용어처럼 원고 밖 지식의 확정에 쓴다.',
  input_schema: {
    type: 'object',
    properties: { query: { type: 'string' } },
    required: ['query'],
    additionalProperties: false,
  },
});

export const RESEARCH_SCHEMA = {
  type: 'object',
  properties: {
    nature: {
      type: 'object',
      description: '글의 성격 — 검토의 강도와 종류를 정하는 판정',
      properties: {
        form: { type: 'string', description: '형식과 그렇게 본 근거' },
        completeness: {
          type: 'object',
          description: '완성도 추정 — 검토 강도를 정하는 분기',
          properties: {
            level: {
              type: 'string',
              enum: ['draft', 'in-revision', 'complete', 'undetermined'],
              description: 'draft=초고 / in-revision=퇴고 중 / complete=완성고 / undetermined=판정 불가',
            },
            note: { type: 'string', description: '그렇게 본 근거' },
          },
          required: ['level', 'note'],
          additionalProperties: false,
        },
        feedbackFit: { type: 'string', description: '이 글에 유효한 검토의 한계' },
      },
      required: ['form', 'completeness', 'feedbackFit'],
      additionalProperties: false,
    },
    voice: {
      type: 'object',
      properties: {
        pov: { type: 'string', description: '시점과 화자 — 신뢰성 포함' },
        conventions: {
          type: 'array',
          description: '문체 관습 — 시제·대사 표기·의도적 문체. 두 곳 이상 반복 확인된 것만',
          items: {
            type: 'object',
            properties: {
              pattern: { type: 'string' },
              evidence: { type: 'array', description: '원고에서 글자 그대로 복사한 인용', items: { type: 'string', verbatim: true } },
            },
            required: ['pattern', 'evidence'],
            additionalProperties: false,
          },
        },
      },
      required: ['pov', 'conventions'],
      additionalProperties: false,
    },
    names: {
      type: 'array',
      description: '인물·설정 용어. 같은 인물의 호칭·애칭은 한 항목으로 묶는다',
      items: {
        type: 'object',
        properties: {
          name: { type: 'string' },
          aliases: { type: 'array', items: { type: 'string' } },
          note: { type: 'string', description: '오인 방지에 필요한 한 줄. 없으면 빈 문자열' },
        },
        required: ['name', 'aliases', 'note'],
        additionalProperties: false,
      },
    },
    premise: {
      type: 'object',
      description: '독자 전제 — 판정 권위는 검색이다',
      properties: {
        sourceWork: {
          type: 'object',
          properties: {
            status: {
              type: 'string',
              enum: ['identified', 'not-identified', 'undetermined'],
              description: 'identified=검색으로 원작 특정 / not-identified=검색했으나 미특정(오리지널 취급) / undetermined=검색 불능',
            },
            name: { type: 'string', description: '특정된 원작명. 아니면 빈 문자열' },
            brief: { type: 'string', description: '원작 배경 브리프. 아니면 빈 문자열' },
          },
          required: ['status', 'name', 'brief'],
          additionalProperties: false,
        },
        genreConventions: {
          type: 'string',
          description: '이 글이 속한 장르·형식의 문법 — 장르 독자가 전제하고 읽는 것. 없으면 빈 문자열',
        },
        seriesContext: { type: 'string', description: '연작·전편 시사와 근거. 없으면 빈 문자열' },
      },
      required: ['sourceWork', 'genreConventions', 'seriesContext'],
      additionalProperties: false,
    },
    boundaries: {
      type: 'array',
      description: '분석 제외 구간 — 머리말·후기·설정 정리·커미션 표기',
      items: {
        type: 'object',
        properties: {
          startQuote: { type: 'string', verbatim: true, description: '구간 시작 문구를 원고에서 그대로 인용' },
          endQuote: { type: 'string', verbatim: true, description: '구간 끝 문구를 원고에서 그대로 인용' },
          reason: { type: 'string' },
        },
        required: ['startQuote', 'endQuote', 'reason'],
        additionalProperties: false,
      },
    },
    unverified: {
      type: 'array',
      description: '확신하지 못한 전제의 명시 목록. 버리지 말고 노출하라',
      items: { type: 'string' },
    },
  },
  required: ['nature', 'voice', 'names', 'premise', 'boundaries', 'unverified'],
  additionalProperties: false,
};

export const PLAN_SCHEMA = {
  type: 'object',
  properties: {
    intent: { type: 'string', description: '이 글이 하려는 것. 한두 문장' },
    protected: {
      type: 'array',
      description: '검토가 건드리면 안 되는 핵심 기법. 본문이 실제로 수행하는 것만. 최대 8개',
      items: {
        type: 'object',
        properties: {
          technique: { type: 'string' },
          evidence: {
            type: 'array',
            description: 'read로 채집한 원고 인용 — 열람 범위 검증 대상',
            items: { type: 'string', verbatim: true },
          },
          rationale: { type: 'string', description: '이 기법이 무엇을 위해 기능하는가' },
        },
        required: ['technique', 'evidence', 'rationale'],
        additionalProperties: false,
      },
    },
    axes: {
      type: 'array',
      // 개수 언급이 없으면 모델이 4~5개라는 자기 규범으로 좁힌다(thinking 실측) — 명시로 중화한다.
      // 층위 경계는 라운드 4 실측 반영: plan층 지적의 33%가 "굳이" 판정을 받았고 대부분 문면
      // 계열 축에서 나왔다. 문면은 교열 단계가 전문으로 소유한다.
      description:
        '원고 전체에 적용할 검토 관점. 개수는 이 글의 위험 프로파일이 정한다 — 개수 자체를 조정 목표로 삼지 마라. 축은 장면·구조·서술 층위를 겨눈다 — 어휘·문장부호·단일 문장 손질은 문면 교열(별도 단계)의 소유라 축으로 세우지 마라',
      items: {
        type: 'object',
        properties: {
          label: { type: 'string', description: '관점의 이름, 20자 이내 명사구 — 특정 대목의 결함명 금지. EXECUTE의 enum 값이 된다' },
          inquiry: {
            type: 'string',
            description: '전문에 적용할 질문형 검토 지시. 결론·대목 열거 금지 — 심으면 검토가 그 자리들의 검산으로 줄어든다',
          },
          risk: { type: 'string', description: '왜 이 글에서 이 관점이 실제로 위험한가' },
          readerCost: {
            type: 'string',
            description:
              '이 위험이 실현되면 독자가 무엇을 치르는가(되읽기, 귀속 혼동, 몰입 이탈, 긴장 소실 등). "X가 정합한가" 같은 확인 질문은 축이 아니다 — 확인이 아니라 비용을 겨눠라. 무혐의로 끝나도 좋다: 그때 무혐의의 뜻은 "모순 없음"이 아니라 "이 비용이 발생하지 않음"이다',
          },
          expectedFinding: {
            type: 'string',
            description:
              '이 축이 유효하다면 나올 법한 지적의 형태 한 건 — 어떤 성격의 대목에서 어떤 결함이 걸릴지. 특정 대목의 결론 예고가 아니라 지적의 생김새다. 이 칸을 채울 수 없는 축은 검토 축이 아니라 확인 절차다',
          },
          evidence: {
            type: 'array',
            description: '위험을 목격한 자리의 예시 인용 — 지적 예정지 목록이 아니다',
            items: { type: 'string', verbatim: true },
          },
          conventionsCheck: {
            type: 'string',
            description:
              '위험 전제와 규약의 대조(무조건): 규약의 어느 항목과 대조했는지, 규약이 침묵해 무엇을 검색으로 확정했는지, 또는 왜 관습과 무관한 전제인지. 대조할 항목이 규약에 없으면 그렇다고 쓴다',
          },
          conventionsBasis: {
            type: 'string',
            enum: ['charter', 'search', 'unrelated', 'unresolved'],
            description:
              'conventionsCheck의 근거 출처: charter=규약 항목과 대조해 확정 / search=규약이 침묵해 직접 검색으로 확정(원장과 대조된다) / unrelated=관습 질문이 아닌 전제 / unresolved=관습 질문인데 확정하지 못함',
          },
        },
        required: ['label', 'inquiry', 'risk', 'readerCost', 'expectedFinding', 'evidence', 'conventionsCheck', 'conventionsBasis'],
        additionalProperties: false,
      },
    },
    verifications: {
      type: 'array',
      description: '이번 라운드에 도구로 확정한 것. tools는 원장과 대조된다 — 쓰지 않은 도구의 신고는 반려',
      items: {
        type: 'object',
        properties: {
          question: { type: 'string' },
          tools: {
            type: 'array',
            description: '사용한 도구 전부',
            items: { type: 'string', enum: ['search', 'grep', 'read'] },
          },
          detail: { type: 'string', description: '검색 질의·grep 패턴·열람 범위' },
          conclusion: { type: 'string' },
        },
        required: ['question', 'tools', 'detail', 'conclusion'],
        additionalProperties: false,
      },
    },
    reviewResponses: {
      type: 'array',
      description: '검수 발견별 응답 원장. 침묵 처분 금지 — 발견마다 한 항목. 초안에서는 빈 배열',
      items: {
        type: 'object',
        properties: {
          target: { type: 'string', description: '응답하는 발견의 target' },
          disposition: {
            type: 'string',
            enum: ['adopted', 'partial', 'rejected'],
            description: 'adopted=반영 / partial=일부 반영 / rejected=기각',
          },
          reason: { type: 'string' },
        },
        required: ['target', 'disposition', 'reason'],
        additionalProperties: false,
      },
    },
  },
  required: ['intent', 'protected', 'axes', 'verifications', 'reviewResponses'],
  additionalProperties: false,
};

// 축 enum이 스테이지·문서마다 다르므로 호출부가 짓는다. 암묵 추가는 없다.
export const findingSchema = (axes: string[]) => {
  if (axes.length === 0) throw new Error('findingSchema: 축이 비어 있다');
  return {
    type: 'object',
    properties: {
      axis: { type: 'string', enum: axes },
      quoteStart: {
        type: 'string',
        verbatim: true,
        description: '대목 시작 문구를 원고에서 그대로 인용 (한 문장 안팎, 위치가 유일하게 식별되게)',
      },
      quoteEnd: { type: 'string', verbatim: true, description: '끝 문구를 원고에서 그대로 인용 (한 문장 안팎, 위치가 유일하게 식별되게)' },
      intent: { type: 'string', description: '이 대목이 하려는 일을 작가의 편에서 읽어낸 것' },
      observation: { type: 'string', description: '읽으면서 무엇이 걸렸는가' },
      cause: { type: 'string', description: '본문의 어떤 구조가 그렇게 만드는가' },
      direction: { type: 'string', description: '30분 안에 시도할 수 있을 만큼 구체적으로. 문장 대필 금지' },
      evidence: { type: 'string', description: '이 지적이 성립하는 본문 근거 — 무엇이 그것을 보여주는가' },
      stake: {
        type: 'string',
        description:
          '반영하지 않으면 독자가 치르는 비용. 크면 크다고, 작으면 작다고 정직하게 — 비용이 작다는 이유로 지적을 접지 마라, 취사선택은 작가의 몫이다',
      },
      manuscriptBasis: {
        type: 'string',
        enum: ['grep', 'reread', 'local'],
        description:
          '원고 대조의 방식: grep=전문 대조 수행(원장과 대조된다) / reread=구간 재열람으로 대조 / local=단일 지점 완결이라 대조 불요',
      },
      manuscriptCheck: {
        type: 'string',
        description:
          '원고 대조 서술(무조건): 다른 곳과 대조해야 성립하는 주장이면 무엇을 확인했는지, 불필요하면 왜인지. "없다·빠졌다"류 주장은 무매치만으로 성립하지 않는다 — 변형 다중 검색과 구간 열람 확인을 함께 서술하라',
      },
      conventionsCheck: {
        type: 'string',
        description:
          '규약 대조 서술(무조건): 이 걸림이 규약의 장르 문법·원작 관습·문체 관습으로 설명될 가능성을 규약의 어느 항목과 대조했는지. 관습으로 설명되면 지적을 접어라. 대조할 정보가 규약에 없으면 그렇다고 써라',
      },
    },
    required: [
      'axis',
      'quoteStart',
      'quoteEnd',
      'intent',
      'observation',
      'cause',
      'direction',
      'evidence',
      'stake',
      'manuscriptBasis',
      'manuscriptCheck',
      'conventionsCheck',
    ],
    additionalProperties: false,
  };
};

export const STRENGTH_SCHEMA = {
  type: 'object',
  properties: {
    quoteStart: { type: 'string', verbatim: true, description: '대목 시작 문구를 원고에서 그대로 인용' },
    quoteEnd: { type: 'string', verbatim: true, description: '끝 문구를 원고에서 그대로 인용' },
    principle: { type: 'string', description: '무엇이 왜 작동하는지. 조언·확장 제안 금지 — 총평의 재료다' },
  },
  required: ['quoteStart', 'quoteEnd', 'principle'],
  additionalProperties: false,
};

// category는 코드가 축에서 파생하므로 받지 않는다.
export const COMPOSE_SCHEMA = {
  type: 'object',
  properties: {
    feedbacks: {
      type: 'array',
      description: '병합은 같은 축·인접 앵커끼리만',
      items: {
        type: 'object',
        properties: {
          findingIndexes: { type: 'array', description: '이 피드백이 옮기는 지적 번호들', items: { type: 'number' } },
          body: { type: 'string' },
        },
        required: ['findingIndexes', 'body'],
        additionalProperties: false,
      },
    },
  },
  required: ['feedbacks'],
  additionalProperties: false,
};

// 구 파이프라인과 공유하던 report_review에 cleared(검토했으나 지적 없음) 층을 더한 에디토리얼 전용판.
// 지적 0건 축을 총평이 말할 수 있어야 "읽고도 지적하지 않았음"이 작가에게 전달된다.
export const COMPOSE_REVIEW_SCHEMA = {
  type: 'object',
  properties: {
    characterization: { type: 'string' },
    strengths: {
      type: 'array',
      description: '최대 5개. 주어진 강점 후보에서 고르고 새로 만들지 말 것',
      items: {
        type: 'object',
        properties: {
          body: { type: 'string', description: '무엇이 어떻게 작동하는지' },
          quoteStart: { type: 'string', verbatim: true, description: '후보로 주어진 인용을 그대로 옮길 것' },
          quoteEnd: { type: 'string', verbatim: true, description: '후보로 주어진 인용을 그대로 옮길 것' },
        },
        required: ['body', 'quoteStart', 'quoteEnd'],
        additionalProperties: false,
      },
    },
    cleared: {
      type: 'array',
      description: '검토했으나 지적이 없었던 관점. 입력 <검토 관점>에서 지적 0건인 축만',
      items: {
        type: 'object',
        properties: {
          axis: { type: 'string', description: '계획의 관점 라벨을 그대로 옮길 것' },
          note: { type: 'string', description: '무엇을 확인했고 왜 지적이 없는지 한두 문장. 새 지적을 만들지 말 것' },
        },
        required: ['axis', 'note'],
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
  required: ['characterization', 'strengths', 'cleared', 'patterns', 'priority'],
  additionalProperties: false,
};

// 검수(OpenAI structured output). confidence는 두지 않는다 — 소비자가 없고 실측 분포에 변별이 없었다.
export const PLAN_REVIEW_SCHEMA_V2 = {
  name: 'plan_review',
  strict: true,
  schema: {
    type: 'object',
    properties: {
      findings: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            kind: { type: 'string', enum: ['over-protection', 'missing-axis', 'biased-axis', 'weak-axis', 'contract-violation'] },
            target: { type: 'string' },
            rationale: { type: 'string' },
            conventionsCheck: { type: 'string' },
            fix: { type: 'string' },
            blocking: { type: 'boolean', description: '이대로 실행되면 검토 결과가 실제로 오염되는가. 개선 권고면 false' },
          },
          required: ['kind', 'target', 'rationale', 'conventionsCheck', 'fix', 'blocking'],
          additionalProperties: false,
        },
      },
      verdict: { type: 'string', enum: ['approve', 'needs-attention'] },
    },
    required: ['findings', 'verdict'],
    additionalProperties: false,
  },
} as const;
