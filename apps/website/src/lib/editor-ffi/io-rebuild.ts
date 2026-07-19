// 'io' 후보 수정(관찰 연속성)의 순수 결정 로직. Page.svelte의 $effect가 이 헬퍼로
// (1) 리사이즈가 관찰 지오메트리를 실제로 바꾸는지, (2) 재빌드 스케줄링을 trailing 디바운스로
// 접을지 결정한다. 타이머·DOM 없이 순수하게 유지해 단위 테스트가 가능하도록 분리했다.

// 툴바 collapse(≈10%)는 재빌드를 유발하지 않고, 회전/분할화면(큰 델타)만 재빌드한다.
export const IO_REBUILD_HEIGHT_THRESHOLD = 0.15;
export const IO_REBUILD_DEBOUNCE_MS = 300;

// 마지막으로 빌드된 높이 대비 새 높이의 상대 변화가 임계값 이상일 때만 재빌드한다.
// 첫 빌드(lastBuiltH<=0)는 항상 재빌드한다.
export const shouldRebuildForResize = (lastBuiltH: number, newH: number, threshold = IO_REBUILD_HEIGHT_THRESHOLD): boolean => {
  if (lastBuiltH <= 0) return true;
  return Math.abs(newH - lastBuiltH) / lastBuiltH >= threshold;
};

export type Cancel = () => void;
export type ScheduleFn = (fn: () => void, ms: number) => Cancel;

// trailing 디바운스: 연속 호출을 마지막 한 번의 실행으로 접는다(이전 예약은 취소).
export const createTrailingDebounce = (schedule: ScheduleFn, ms: number = IO_REBUILD_DEBOUNCE_MS) => {
  let cancel: Cancel | undefined;
  return {
    call(fn: () => void): void {
      cancel?.();
      cancel = schedule(() => {
        cancel = undefined;
        fn();
      }, ms);
    },
    cancel(): void {
      cancel?.();
      cancel = undefined;
    },
  };
};
