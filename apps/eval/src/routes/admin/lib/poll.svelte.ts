type PollOptions = {
  // 매 인터벌마다 평가해서 폴링을 켜고 끈다(예: run.status !== 'running'이면 중단). 생략 시 항상 켜짐.
  enabled?: () => boolean;
};

// 컴포넌트 초기화 중(또는 다른 $effect 안)에 호출해야 한다 — $effect가 마운트/해제를 관리하므로
// 페이지 이탈 시 자동으로 setInterval이 해제된다. document.hidden이면 틱을 건너뛰어 백그라운드 탭에서 쉰다.
export const usePolling = (fn: () => void | Promise<void>, intervalMs: number, options: PollOptions = {}): void => {
  $effect(() => {
    if (options.enabled?.() === false) return;

    const tick = () => {
      if (document.hidden) return;
      void fn();
    };

    const id = setInterval(tick, intervalMs);
    return () => clearInterval(id);
  });
};
