// 모달 안 교차 참조의 이동 — 대상 카드로 스크롤하고 잠깐 강조한다. 원고로 가는 이동은 없다.
const FLASH_MS = 1200;

export const jumpTo = (anchor: string): void => {
  const el = document.querySelector<HTMLElement>(`#${CSS.escape(anchor)}`);
  if (!el) return;
  el.scrollIntoView({ block: 'center', behavior: 'smooth' });
  el.dataset.flash = '';
  window.setTimeout(() => {
    delete el.dataset.flash;
  }, FLASH_MS);
};
