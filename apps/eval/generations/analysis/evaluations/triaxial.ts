import { note, reason, scale, yesNo } from './fields.ts';
import type { EvaluationSpec } from '../../../core/contracts.ts';

// 세 축은 각각 사실·가치·행동 가능성이다. 질문은 무엇을 재는지가 문장만으로 읽혀야 한다.
export const TRIAXIAL: EvaluationSpec = {
  id: 'triaxial',
  label: '3축 판정',
  stages: [
    {
      key: 'judgment',
      label: '판정',
      run: [
        yesNo('readCorrectly', '이 작품을 제대로 파악했나요?', '작품이 무엇을 하려는 글인지 잘못 봤습니다', '파악'),
        yesNo('priorityUseful', '어디서부터 손댈지 납득되나요?', '제시한 순서에 동의할 수 없습니다', '순서'),
        scale('helpfulness', '전체적으로 도움이 되었나요?', ['전혀', '별로', '보통', '도움됨', '큰 도움']),
        reason('note', '어디가 어떻게 어긋났는지 적어주세요'),
        note('comment', '코멘트 (선택)'),
      ],
      items: [
        {
          match: (item) => item.kind === 'finding',
          fields: [
            yesNo('correct', '본문을 정확히 읽었나요?', '본문에 없는 것을 말했거나 잘못 읽었습니다', '정확'),
            yesNo('needed', '짚을 만한 내용인가요?', '맞는 말이지만 굳이 말할 일은 아닙니다', '가치'),
            yesNo('useful', '작가가 무엇을 할지 알 수 있나요?', '읽어도 어떻게 손대야 할지 모르겠습니다', '실행'),
            reason('note', '어디가 어떻게 어긋났는지 적어주세요'),
          ],
        },
      ],
    },
  ],
};
