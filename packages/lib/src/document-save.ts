import { match } from 'ts-pattern';

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

export const DOCUMENT_SAVE_ACTION_LABELS = {
  leave: { proceed: '나가기', discard: '저장하지 않고 나가기', cancel: '계속 편집' },
  reload: { proceed: '불러오기', discard: '변경사항 버리고 불러오기', cancel: '계속 편집' },
  logout: { proceed: '로그아웃', discard: '저장하지 않고 로그아웃', cancel: '로그아웃 취소' },
  login: { proceed: '로그인', discard: '저장하지 않고 로그인', cancel: '계속 편집' },
  save: { proceed: '확인', discard: undefined, cancel: '계속 편집' },
  sync: { proceed: '확인', discard: undefined, cancel: '계속 편집' },
  quit: { proceed: '종료', discard: '저장하지 않고 종료', cancel: '종료 취소' },
};

export function getDocumentSaveDialogText({
  reason,
  pending,
  unknown = false,
  protectedChanges = false,
  completed = false,
}: {
  reason: keyof typeof DOCUMENT_SAVE_ACTION_LABELS;
  pending: boolean;
  unknown?: boolean;
  protectedChanges?: boolean;
  completed?: boolean;
}): { title: string; description: string } {
  if (completed) {
    const title = {
      leave: '이제 안전하게 나갈 수 있어요',
      reload: '이제 최신 버전을 불러올 수 있어요',
      logout: '이제 안전하게 로그아웃할 수 있어요',
      login: '이제 로그인 화면으로 이동할 수 있어요',
      save: '최근 변경사항을 안전하게 저장했어요',
      sync: '서버에 저장했어요',
      quit: '이제 안전하게 종료할 수 있어요',
    }[reason];
    return {
      title,
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
  const description = match({ reason, protectedChanges, pending, unknown })
    .with({ unknown: true }, () => '최근 변경사항이 안전하게 저장되었는지 확인할 수 없어요.')
    .with(
      { reason: 'sync', protectedChanges: true, pending: true },
      () => '이 기기에는 안전하게 저장되었지만,\n서버와 동기화하는 데 평소보다 오래 걸리고 있어요.',
    )
    .with(
      { reason: 'sync', protectedChanges: true, pending: false },
      () => '이 기기에는 안전하게 저장되었지만,\n서버와 동기화하지 못했어요.',
    )
    .with({ reason: 'sync', pending: true }, () => '최근 변경사항을 서버와 동기화하는 데 평소보다 오래 걸리고 있어요.')
    .with({ reason: 'sync', pending: false }, () => '최근 변경사항을 서버와 동기화하지 못했어요.')
    .with({ reason: 'save', pending: true }, () => '저장이 평소보다 오래 걸리고 있어요.\n최근 변경사항을 저장하는 중이에요.')
    .with({ reason: 'save', pending: false }, () => '최근 변경사항을 안전하게 저장하지 못했어요.')
    .with({ reason: 'reload', pending: true }, () => '저장이 평소보다 오래 걸리고 있어요.\n지금 불러오면 최근 변경사항을 잃을 수 있어요.')
    .with({ reason: 'reload', pending: false }, () => '안전하게 저장하지 못했어요.\n지금 불러오면 최근 변경사항을 잃을 수 있어요.')
    .with(
      { reason: 'login', pending: true },
      () => '저장이 평소보다 오래 걸리고 있어요.\n지금 로그인 화면으로 이동하면 최근 변경사항을 잃을 수 있어요.',
    )
    .with(
      { reason: 'login', pending: false },
      () => '안전하게 저장하지 못했어요.\n지금 로그인 화면으로 이동하면 최근 변경사항을 잃을 수 있어요.',
    )
    .with(
      { reason: 'logout', pending: true },
      () => '저장이 평소보다 오래 걸리고 있어요.\n지금 로그아웃하면 최근 변경사항을 잃을 수 있어요.',
    )
    .with({ reason: 'logout', pending: false }, () => '안전하게 저장하지 못했어요.\n지금 로그아웃하면 최근 변경사항을 잃을 수 있어요.')
    .with({ reason: 'quit', pending: true }, () => '저장이 평소보다 오래 걸리고 있어요.\n지금 종료하면 최근 변경사항을 잃을 수 있어요.')
    .with({ reason: 'quit', pending: false }, () => '안전하게 저장하지 못했어요.\n지금 종료하면 최근 변경사항을 잃을 수 있어요.')
    .with({ reason: 'leave', pending: true }, () => '저장이 평소보다 오래 걸리고 있어요.\n지금 나가면 최근 변경사항을 잃을 수 있어요.')
    .with({ reason: 'leave', pending: false }, () => '안전하게 저장하지 못했어요.\n지금 나가면 최근 변경사항을 잃을 수 있어요.')
    .exhaustive();
  return { title, description };
}
