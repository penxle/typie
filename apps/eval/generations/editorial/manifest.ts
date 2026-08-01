import { TRIAXIAL } from './evaluations/triaxial.ts';
import type { GenerationManifest } from '../../core/contracts.ts';

// planReview를 별도 phase로 둔다. 구 구조에서는 검수 토큰이 plan 행에 섞여 들어가 비용
// 화면이 세대를 알아내 사후 보정해야 했다 — phase가 프롬프트 키와 같아지면 그 어긋남이 없다.
export const EDITORIAL_MANIFEST: GenerationManifest = {
  id: 'editorial',
  label: '에디토리얼',
  status: 'active',
  phases: [
    { key: 'research', label: '원고 조사' },
    { key: 'plan', label: '비평 계획' },
    { key: 'planReview', label: '계획 검수' },
    { key: 'execute', label: '작품 검토' },
    { key: 'local', label: '문면 교열' },
    { key: 'compose', label: '피드백 다듬기' },
    { key: 'composeReview', label: '총평 작성' },
  ],
  itemKinds: [
    { key: 'characterization', label: '작품 파악' },
    { key: 'finding', label: '지적' },
    { key: 'strength', label: '강점' },
    { key: 'cleared', label: '살펴봤지만 문제가 없던 것' },
    { key: 'pattern', label: '되풀이되는 경향' },
    { key: 'priority', label: '먼저 손댈 것' },
  ],
  facets: [
    { key: 'axis', label: '검토 관점' },
    { key: 'layer', label: '층위', groupBy: true },
    { key: 'theme', label: '주제' },
  ],
  evaluations: [TRIAXIAL],
  // 리서치와 비평 계획은 러너가 원장에 남긴다. plan은 검수 회차까지 담고 있어 최종본만 꺼낸다.
  // search 질의는 리서치의 근거 기록이라 함께 싣는다 — read·grep은 원고 접근이라 당연해서 뺀다.
  artifacts: {
    label: '리서치·비평 계획',
    ledgerKeys: ['research', 'plan', 'ledger/research'],
    select: (rows) => {
      const research = rows.research ?? null;
      const plan = (rows.plan as { final?: unknown } | undefined)?.final ?? null;
      const tools = (rows['ledger/research'] as { tools?: { tool: string }[] } | undefined)?.tools ?? [];
      const searches = tools.filter((t) => t.tool === 'search');
      return research && plan ? { research, plan, searches } : null;
    },
  },
};
