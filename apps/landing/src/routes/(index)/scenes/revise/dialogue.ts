export type ReplyBlock =
  | { kind: 'text'; text: string }
  | { kind: 'quote'; text: string }
  | { kind: 'note'; title: string; lines: readonly string[]; entity: string; time: string };

export type Turn = { question: string; tools: readonly string[]; reply: readonly ReplyBlock[] };

export const TURNS: readonly Turn[] = [
  {
    question: '별 이야기가 너무 자주 되풀이되는 것 같은데, 어디를 줄여야 할지 모르겠어요.',
    tools: ['문서를 읽었어요', '문서 구조를 살펴봤어요'],
    reply: [
      {
        kind: 'text',
        text: '별이 되풀이되는 건 의도로 읽혀요. 흩어진 너도, 떠나온 지구도 결국 별이니까요. 문제는 “특별하지 않은 별”이라는 말이 두 군데에서 따로 나온다는 점이에요.',
      },
      { kind: 'quote', text: '하나도 대단치 않고, 하나도 특별할 것 없는 별.' },
      {
        kind: 'text',
        text: '앞선 “특별한 별은 아니었을 거야”를 덜고 이 문장에서 처음 말하게 두면, 뒤이은 고백이 훨씬 세게 닿아요.',
      },
    ],
  },
  {
    question: '그럼 그 방향으로 고칠 것들 노트로 정리해주세요.',
    tools: ['노트를 만들었어요'],
    reply: [
      {
        kind: 'note',
        title: '별 비유 손보기',
        lines: ['특별하지 않은 별은 탈출선 장면에만 남기기', '지구 회상은 “우주는 아득히 넓고”로 시작하기'],
        entity: '최후이자 최초의 너에게',
        time: '몇 초 전',
      },
      {
        kind: 'text',
        text: '노트로 옮겨 뒀어요. 원고 옆에 열어두고 하나씩 지워가면 돼요.',
      },
    ],
  },
];

export type DialogueWindow = { type: readonly [number, number]; question: number; tools: number; blocks: readonly number[] };

// 한 번의 주고받음 — 물음을 치고, 보내고, 도구가 돌고, 답이 문단씩 쌓인다
export const DIALOGUE_WINDOWS: readonly DialogueWindow[] = [
  { type: [0.08, 0.17], question: 0.19, tools: 0.23, blocks: [0.31, 0.4, 0.45] },
  { type: [0.48, 0.55], question: 0.57, tools: 0.61, blocks: [0.66, 0.74] },
];
