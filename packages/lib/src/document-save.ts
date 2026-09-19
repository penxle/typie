import { match, P } from 'ts-pattern';

export type DocumentSaveState = 'protected' | 'synced' | 'pending' | 'failed' | 'sync-failed' | 'unknown';

export type DocumentSaveDocument = {
  id: string;
  title: string;
  icon?: string;
  iconColor?: string;
  location: string;
  status: DocumentSaveState;
  protectedChanges?: boolean;
};

export function getDocumentSaveDialogText({
  reason,
  pending,
  unknown = false,
  protectedChanges = false,
  completed = false,
}: {
  reason: 'leave' | 'reload' | 'logout' | 'login' | 'save' | 'sync';
  pending: boolean;
  unknown?: boolean;
  protectedChanges?: boolean;
  completed?: boolean;
}): { title: string; description: string } {
  if (completed) {
    return {
      title: '저장을 완료했어요',
      description: reason === 'sync' ? '최근 변경사항을 서버에 저장했어요.' : '최근 변경사항을 안전하게 저장했어요.',
    };
  }
  const title = match({ reason, pending, unknown })
    .with({ reason: 'reload', pending: true }, () => '아직 최신 버전을 불러올 수 없어요')
    .with({ reason: 'reload', pending: false }, () => '최신 버전을 불러올 수 없어요')
    .with({ reason: 'login', pending: true }, () => '아직 로그인 화면으로 이동할 수 없어요')
    .with({ reason: 'login', pending: false }, () => '로그인 화면으로 이동할 수 없어요')
    .with({ reason: 'sync', pending: true }, () => '아직 서버에 저장하지 못했어요')
    .with({ reason: 'sync', pending: false }, () => '서버에 저장하지 못했어요')
    .with({ unknown: true }, () => '저장 상태를 확인할 수 없어요')
    .with({ pending: true }, () => '아직 저장을 완료하지 못했어요')
    .with({ pending: false }, () => '저장하지 못했어요')
    .exhaustive();
  const description = match({ reason, protectedChanges })
    .with(
      { reason: 'sync', protectedChanges: true },
      () => '최근 변경사항을 서버에 저장하지 못했어요.\n이 기기에는 안전하게 저장되어 있어요.',
    )
    .with({ reason: 'sync', protectedChanges: false }, () => '최근 변경사항을 서버에 저장하지 못했어요.')
    .with({ reason: P.union('reload', 'login', 'save') }, () => '최근 변경사항을 안전하게 저장하지 못했어요.')
    .with({ reason: 'logout' }, () => '지금 로그아웃하면 최근 변경사항을 잃을 수 있어요.')
    .with({ reason: 'leave' }, () => '지금 닫으면 최근 변경사항을 잃을 수 있어요.')
    .exhaustive();
  return { title, description };
}
