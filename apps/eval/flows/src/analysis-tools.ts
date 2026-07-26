import type OpenAI from 'openai';

// 재설계 파이프라인의 도구 정의. 단계마다 구조화 출력을 tool call로 강제한다.

export const SURVEY_TOOL: OpenAI.Chat.Completions.ChatCompletionFunctionTool = {
  type: 'function',
  function: {
    name: 'report_profile',
    description: '원고를 읽고 파악한 사실을 전달한다.',
    parameters: {
      type: 'object',
      properties: {
        form: { type: 'string', description: '글의 형식과 그렇게 본 근거' },
        isDerivative: { type: 'boolean' },
        derivativeSource: { type: 'string', description: '2차 창작이면 원작. 아니면 빈 문자열' },
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
  },
};

export const REVIEW_TOOL: OpenAI.Chat.Completions.ChatCompletionFunctionTool = {
  type: 'function',
  function: {
    name: 'report_findings',
    description: '읽으면서 걸린 지점들을 전달한다.',
    parameters: {
      type: 'object',
      properties: {
        findings: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              quoteStart: { type: 'string', description: '대목의 시작 문구를 원고에서 그대로 인용 (15자 안팎)' },
              quoteEnd: { type: 'string', description: '끝 문구를 원고에서 그대로 인용 (15자 안팎)' },
              kind: { type: 'string', enum: ['error', 'readability', 'structure', 'strength'] },
              intent: { type: 'string', description: '이 대목이 하려는 일을 작가의 편에서 읽어낸 것' },
              observation: { type: 'string', description: '읽으면서 무엇이 걸렸는가' },
              cause: { type: 'string', description: '본문의 어떤 구조가 그렇게 만드는가' },
              direction: { type: 'string', description: '30분 안에 시도할 수 있을 만큼 구체적으로. 문장을 대신 써주지 말 것' },
              evidence: { type: 'string' },
            },
            required: ['quoteStart', 'quoteEnd', 'kind', 'intent', 'observation', 'cause', 'direction', 'evidence'],
            additionalProperties: false,
          },
        },
      },
      required: ['findings'],
      additionalProperties: false,
    },
  },
};

export const DEDUPE_TOOL: OpenAI.Chat.Completions.ChatCompletionFunctionTool = {
  type: 'function',
  function: {
    name: 'report_groups',
    description: '같은 문제를 말하는 지적끼리 묶어 전달한다.',
    parameters: {
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
  },
};

export const VERIFY_TOOL: OpenAI.Chat.Completions.ChatCompletionFunctionTool = {
  type: 'function',
  function: {
    name: 'report_verdict',
    description: '지적의 타당성을 판정한다.',
    parameters: {
      type: 'object',
      properties: {
        anchors: {
          type: 'array',
          description: '위치마다 하나씩. 주어진 위치 수와 같아야 한다',
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
      required: ['anchors'],
      additionalProperties: false,
    },
  },
};

export const COMPOSE_TOOL: OpenAI.Chat.Completions.ChatCompletionFunctionTool = {
  type: 'function',
  function: {
    name: 'report_feedbacks',
    description: '작가가 읽을 피드백 목록을 전달한다.',
    parameters: {
      type: 'object',
      properties: {
        feedbacks: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              groupIndex: { type: 'integer' },
              category: { type: 'string', description: '2~6자 한국어' },
              polarity: { type: 'string', enum: ['issue', 'highlight'] },
              body: { type: 'string' },
            },
            required: ['groupIndex', 'category', 'polarity', 'body'],
            additionalProperties: false,
          },
        },
      },
      required: ['feedbacks'],
      additionalProperties: false,
    },
  },
};

export const COMPOSE_REVIEW_TOOL: OpenAI.Chat.Completions.ChatCompletionFunctionTool = {
  type: 'function',
  function: {
    name: 'report_review',
    description: '작품 전체에 대한 판단을 전달한다.',
    parameters: {
      type: 'object',
      properties: {
        characterization: { type: 'string' },
        strengths: { type: 'string' },
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
  },
};
