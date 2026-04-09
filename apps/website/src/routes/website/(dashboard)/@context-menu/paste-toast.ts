import { TypieError } from '@typie/lib/errors';
import { Toast } from '@typie/ui/notification';
import { unwrapError } from '$lib/graphql';

const defaultPasteErrorMessage = '붙여넣기 중 오류가 발생했어요';

const pasteErrorMessages: Record<string, string> = {
  site_mismatch: '이 위치에는 붙여넣을 수 없어요.',
  circular_reference: '자기 자신 또는 하위 항목 안에는 붙여넣을 수 없어요.',
  paste_source_not_found: '붙여넣을 항목을 찾을 수 없어요.',
  character_count_limit_exceeded: '현재 플랜의 글자 수 제한을 초과했어요.',
  blob_size_limit_exceeded: '현재 플랜의 파일 크기 제한을 초과했어요.',
};

export function getPasteErrorMessage(err: unknown): string {
  const error = unwrapError(err);
  if (!(error instanceof TypieError)) {
    return defaultPasteErrorMessage;
  }

  return pasteErrorMessages[error.code] || defaultPasteErrorMessage;
}

export function showPasteToast(promise: Promise<unknown>, count: number): Promise<unknown> {
  return Toast.promise(promise, {
    loading: `${count}개의 항목을 붙여넣는 중이에요`,
    success: `${count}개의 항목을 붙여넣었어요`,
    error: getPasteErrorMessage,
  });
}
