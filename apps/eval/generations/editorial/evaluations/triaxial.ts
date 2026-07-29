import { choice, note, reason, reasonFor, reasonKind, scale, triState, yesNo } from './fields.ts';
import type { EvaluationSpec } from '../../../core/contracts.ts';
import type { ChoiceOption } from './fields.ts';

// 라운드 3 정성 노트가 수렴한 아니오 유형. 이 분류가 있어야 세대 개선(리서치·규약 대조)이
// 어느 유형의 오탐을 줄였는지 수작업 독해 없이 읽힌다.
export const NO_REASONS: ChoiceOption[] = [
  { value: 'misread', label: '본문 오독' },
  { value: 'convention', label: '원작·관습 문제' },
  { value: 'taste', label: '취향·스타일 강요' },
  { value: 'trivial', label: '사소함' },
  { value: 'other', label: '기타' },
];

// 하단 종합 평가 폼에 두는 필드 — 평가자 배경 신고와, 피드백 전부를 보고 나서야 매길 수 있는
// 것들(놓친 것·일관·도움도·코멘트). 배경은 총평에 대한 판정이 아니라 총평 탭에 둘 수 없다.
// 나머지 첫 단계 run 필드는 총평 탭에 들어간다.
export const RUN_FOOTER_KEYS = new Set(['sourceFamiliarity', 'missed', 'consistent', 'consistentNote', 'helpfulness', 'comment']);

// 세 축은 각각 사실·가치·행동 가능성이다. 질문은 무엇을 재는지가 문장만으로 읽혀야 한다.
// 도움도와 지적 세 축의 문구는 라운드 3과 같아야 한다 — 라운드 간 비교가 이 문구 위에 서 있다.
export const TRIAXIAL: EvaluationSpec = {
  id: 'triaxial',
  label: '3축 판정',
  stages: [
    {
      key: 'judgment',
      label: '작품 판정',
      run: [
        // 글의 분류(2차창작 여부)가 아니라 평가자의 상태를 묻는다 — 판정 해석에 필요한 것은
        // "이 평가자가 이 글이 전제하는 배경을 알고 읽었는가"다. 다른 문항이 전부 오라클의
        // 산출물을 묻기 때문에, 주어를 명시하지 않으면 이것도 오라클 얘기로 읽힌다.
        choice(
          'sourceFamiliarity',
          '평가자님은 이 글의 배경(원작·설정)을 알고 계신가요?',
          [
            { value: 'known', label: '알아요' },
            { value: 'unknown', label: '몰라요' },
            { value: 'none', label: '알 필요 없어요' },
          ],
          '배경',
        ),
        yesNo('priorityUseful', '어디서부터 손댈지 납득되나요?', '제시한 순서에 동의할 수 없습니다', '순서'),
        reason('note', '어디가 어떻게 어긋났는지 적어주세요'),
        // 하단 폼 순서 = 배열 순서: 구조화 문항(예/아니오→척도)이 위, 자유 서술이 아래.
        yesNo('consistent', '피드백이 서로 모순 없이 일관되나요?', '같은 대목을 두고 서로 충돌하는 피드백이 있습니다', '일관'),
        reasonFor('consistentNote', 'consistent', '어느 피드백끼리 충돌하는지 적어주세요'),
        scale('helpfulness', '전체적으로 도움이 되었나요?', ['전혀', '별로', '보통', '도움됨', '큰 도움']),
        note('missed', '짚었어야 하는데 놓친 것 (선택)'),
        note('comment', '코멘트 (선택)'),
      ],
      items: [
        {
          match: (item) => item.kind === 'finding',
          fields: [
            yesNo('correct', '본문을 정확히 읽었나요?', '본문에 없는 것을 말했거나 잘못 읽었습니다', '정확'),
            yesNo('needed', '짚을 만한 내용인가요?', '맞는 말이지만 굳이 말할 일은 아닙니다', '가치'),
            yesNo('useful', '작가가 무엇을 할지 알 수 있나요?', '읽어도 어떻게 손대야 할지 모르겠습니다', '실행'),
            reasonKind('reasonKind', NO_REASONS),
            reason('note', '어디가 어떻게 어긋났는지 적어주세요'),
          ],
        },
        {
          match: (item) => item.kind === 'characterization',
          fields: [
            triState(
              'readCorrectly',
              '이 작품을 제대로 파악했나요?',
              '작품이 무엇을 하려는 글인지 잘못 봤습니다',
              '배경을 몰라 판단 어려움',
              '파악',
            ),
            reason('note', '어디가 어떻게 어긋났는지 적어주세요'),
          ],
        },
        {
          match: (item) => item.kind === 'strength',
          fields: [
            yesNo('agree', '이 강점에 동의하나요?', '강점이라기보다 과찬이거나 단순 감상입니다', '동의'),
            reason('note', '왜 강점이 아닌지 적어주세요'),
          ],
        },
        {
          match: (item) => item.kind === 'cleared',
          fields: [
            triState('clear', '이 관점, 정말 문제가 없다고 보나요?', '살펴보면 문제가 있는 관점입니다', '모르겠음', '무혐의'),
            reason('note', '어떤 문제가 있는지 적어주세요'),
          ],
        },
      ],
    },
    {
      // 작품 판정이 확정된 뒤에야 열린다 — 리서치·계획을 먼저 보면 작품 판정이 오염된다.
      key: 'artifacts',
      label: '리서치·비평 계획 평가',
      run: [
        triState(
          'researchAccurate',
          '리서치가 작품·원작을 맞게 파악했나요?',
          '조사가 틀렸거나 다른 작품으로 읽었습니다',
          '판단 어려움',
          '리서치',
        ),
        reasonFor('researchNote', 'researchAccurate', '리서치의 어디가 어긋났는지 적어주세요'),
        yesNo('planApt', '검토 관점들이 이 글에 맞는 관점인가요?', '이 글과 맞지 않는 관점이 있습니다', '계획'),
        reasonFor('planNote', 'planApt', '어느 관점이 왜 맞지 않는지 적어주세요'),
        choice(
          'trustChange',
          '리서치·계획을 보고 나니 피드백이 더 신뢰되나요?',
          [
            { value: 'more', label: '더 신뢰' },
            { value: 'same', label: '그대로' },
            { value: 'less', label: '덜 신뢰' },
          ],
          '신뢰',
        ),
        note('revisit', '바꾸고 싶어진 판정이 있다면 — 기존 판정과 그 이유 (선택)'),
        note('artifactComment', '리서치·비평 계획 코멘트 (선택)'),
      ],
      items: [],
    },
  ],
};
