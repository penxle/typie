import { css } from '@typie/styled-system/css';

// 기존 화면에서 귀납한 관용구. 화면마다 다시 쓰면 또 어긋나므로 한곳에 못박는다.

// Panda는 정적 추출기다 — 값이 함수 인자로 들어오면 그 선언을 만들지 못한다.
// 폭을 매개변수로 받는 헬퍼를 쓰면 max-width가 통째로 사라져 전 화면이 전폭으로 퍼진다.
export const pageClass = css({ maxWidth: '1080px', marginX: 'auto', paddingY: '40px', paddingX: '32px' });
export const narrowPageClass = css({ maxWidth: '760px', marginX: 'auto', paddingY: '40px', paddingX: '32px' });

export const pageTitleClass = css({ fontSize: '22px', fontWeight: 'bold' });
export const pageDescClass = css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' });
export const pageHeaderClass = css({ marginBottom: '20px' });

// 기존 화면 전체가 이 카드 위에 산다 — 어드민 셸 배경이 surface.subtle이고 카드가
// surface.default라, 카드가 없으면 화면이 통째로 회색 바탕에 글자만 뜬 것처럼 보인다.
export const cardClass = css({
  backgroundColor: 'surface.default',
  borderWidth: '1px',
  borderColor: 'border.default',
  borderRadius: '12px',
  padding: '20px',
  boxShadow: 'small',
});

// 표를 담는 카드는 패딩 없이 모서리만 잘라낸다.
export const panelClass = css({
  backgroundColor: 'surface.default',
  borderWidth: '1px',
  borderColor: 'border.default',
  borderRadius: '12px',
  boxShadow: 'small',
  overflow: 'hidden',
});

// 카드를 세로로 쌓는 화면(평가자·라운드 상세)은 카드가 자기 아래 여백을 들고 있다.
export const sectionCardClass = css({
  backgroundColor: 'surface.default',
  borderWidth: '1px',
  borderColor: 'border.default',
  borderRadius: '12px',
  padding: '20px',
  boxShadow: 'small',
  marginBottom: '16px',
});

export const cardTitleClass = css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle', marginBottom: '12px' });

// 폼 카드의 라벨·입력. 목록 표의 13px보다 한 단계 크다 — 입력은 읽는 것이 아니라 다루는 것이다.
export const formLabelClass = css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' });

export const formInputClass = css({
  width: 'full',
  paddingX: '10px',
  paddingY: '8px',
  borderWidth: '1px',
  borderColor: 'border.default',
  borderRadius: '8px',
  fontSize: '14px',
  backgroundColor: 'surface.default',
  transition: '[border-color 0.15s ease]',
  _hover: { borderColor: 'border.strong' },
});

// 커스텀 체크박스 — 기본 체크박스는 브랜드색을 받지 못해 폼 안에서 혼자 회색으로 남는다.
export const checkboxClass = css({
  appearance: 'none',
  width: '16px',
  height: '16px',
  borderWidth: '1px',
  borderColor: 'border.strong',
  borderRadius: '4px',
  backgroundColor: 'surface.default',
  cursor: 'pointer',
  flexShrink: '0',
  transition: '[background-color 0.15s ease, border-color 0.15s ease]',
  _checked: { backgroundColor: 'accent.brand.default', borderColor: 'border.brand' },
});

// 폼 제출 결과 자리. 높이를 고정해 문구가 뜰 때 아래가 밀리지 않게 한다.
export const formNoticeClass = css({ marginTop: '8px', height: '16px', fontSize: '12px' });

export const formSubmitClass = css({
  paddingX: '16px',
  paddingY: '10px',
  borderRadius: '8px',
  backgroundColor: 'accent.brand.default',
  color: 'text.bright',
  fontSize: '13px',
  fontWeight: 'bold',
  transition: '[background-color 0.15s ease]',
  _disabled: { backgroundColor: 'interactive.disabled', cursor: 'not-allowed' },
  ['&:hover:not(:disabled)']: { backgroundColor: 'accent.brand.hover' },
  ['&:not(:disabled)']: { cursor: 'pointer' },
});

export const chipClass = css({
  paddingX: '8px',
  paddingY: '2px',
  borderRadius: 'full',
  fontSize: '12px',
  fontWeight: 'medium',
  backgroundColor: 'surface.muted',
  color: 'text.subtle',
});

export const attentionChipClass = css({
  paddingX: '8px',
  paddingY: '2px',
  borderRadius: 'full',
  fontSize: '12px',
  fontWeight: 'medium',
  backgroundColor: 'accent.warning.subtle',
  color: 'accent.warning.default',
});

export const adminChipClass = css({
  paddingX: '8px',
  paddingY: '2px',
  borderRadius: 'full',
  fontSize: '11px',
  fontWeight: 'medium',
  backgroundColor: 'accent.brand.subtle',
  color: 'accent.brand.default',
});

// 수치를 세로로 견주는 표 — 첫 열만 왼쪽, 나머지는 오른쪽 정렬한다.
export const numericTableClass = css({
  width: 'full',
  fontSize: '13px',
  fontVariantNumeric: 'tabular-nums',
  '& td, & th': { paddingX: '10px', paddingY: '8px', textAlign: 'left' },
  '& td:not(:first-child), & th:not(:first-child)': { textAlign: 'right' },
});

export const sectionTitleClass = css({ fontSize: '16px', fontWeight: 'bold', marginBottom: '12px' });

// 주요 동작. 비활성은 interactive.disabled로 떨어뜨린다.
export const primaryButtonClass = css({
  paddingX: '14px',
  paddingY: '9px',
  borderRadius: '8px',
  backgroundColor: 'accent.brand.default',
  color: 'text.bright',
  fontSize: '13px',
  fontWeight: 'bold',
  transition: '[background-color 0.15s ease]',
  _hover: { backgroundColor: 'accent.brand.hover' },
  _disabled: { backgroundColor: 'interactive.disabled', color: 'text.disabled', cursor: 'not-allowed' },
  ['&:not(:disabled)']: { cursor: 'pointer' },
});

export const outlineButtonClass = css({
  paddingX: '14px',
  paddingY: '9px',
  borderWidth: '1px',
  borderColor: 'border.default',
  borderRadius: '8px',
  fontSize: '13px',
  color: 'text.subtle',
  transition: '[background-color 0.15s ease]',
  _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
  ['&:hover:not(:disabled)']: { backgroundColor: 'surface.muted' },
  ['&:not(:disabled)']: { cursor: 'pointer' },
});

export const quietButtonClass = css({
  fontSize: '12px',
  color: 'text.subtle',
  transition: '[color 0.15s ease]',
  _hover: { color: 'text.default' },
  _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
  ['&:not(:disabled)']: { cursor: 'pointer' },
});

export const dangerButtonClass = css({
  fontSize: '12px',
  color: 'text.danger',
  cursor: 'pointer',
  transition: '[color 0.15s ease]',
  _hover: { color: 'accent.danger.hover' },
});

// 목록 마지막 칸의 상세 링크.
export const rowLinkClass = css({ fontSize: '12px', color: 'text.subtle', _hover: { color: 'text.default' } });

export const tableClass = css({
  width: 'full',
  fontSize: '13px',
  '& td, & th': { paddingX: '16px', paddingY: '10px', textAlign: 'left' },
});

export const tableHeadClass = css({
  '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
});

export const tableRowClass = css({ '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' } });

export const inputClass = css({
  paddingX: '10px',
  paddingY: '8px',
  borderWidth: '1px',
  borderColor: 'border.default',
  borderRadius: '8px',
  fontSize: '13px',
  backgroundColor: 'surface.default',
  transition: '[border-color 0.15s ease]',
  _focus: { borderColor: 'border.strong' },
});

export const fieldLabelClass = css({ fontSize: '12px', color: 'text.faint' });

export const emptyClass = css({ paddingY: '48px', textAlign: 'center', fontSize: '14px', color: 'text.faint' });

export const noticeClass = css({
  padding: '12px',
  borderRadius: '8px',
  fontSize: '13px',
  backgroundColor: 'accent.danger.subtle',
  color: 'text.danger',
});

export const successNoticeClass = css({
  padding: '12px',
  borderRadius: '8px',
  fontSize: '13px',
  backgroundColor: 'accent.success.subtle',
  color: 'text.default',
});
