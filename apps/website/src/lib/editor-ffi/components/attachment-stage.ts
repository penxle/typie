import type { LocalUpload } from '../types';

export type AttachmentStage = 'ready' | 'localActive' | 'localFailed' | 'serverPending' | 'unresolved' | 'empty';

export const attachmentStage = ({
  hasAsset,
  localPhase,
  hasId,
  unresolved,
}: {
  hasAsset: boolean;
  localPhase: LocalUpload['phase'] | undefined;
  hasId: boolean;
  unresolved: boolean;
}): AttachmentStage => {
  if (hasAsset) return 'ready';
  if (localPhase === 'active') return 'localActive';
  if (localPhase === 'failed') return 'localFailed';
  if (!hasId) return 'empty';
  // `serverPending`은 곧 채워질 수 있으므로 어포던스를 주지 않는다. `missing`(=unresolved)은 서버가 확정한
  // 부재라 노드를 지우지 않고 빈 placeholder와 같은 모습·어포던스를 준다(같은 ID를 재예약해 이어받는다).
  return unresolved ? 'unresolved' : 'serverPending';
};

/** 빈 placeholder와 미해석 노드는 사용자에게 구별되지 않는다 — 파일 선택·삭제가 둘 다에 열린다. */
export const isEmptyLikeStage = (stage: AttachmentStage): boolean => stage === 'empty' || stage === 'unresolved';
