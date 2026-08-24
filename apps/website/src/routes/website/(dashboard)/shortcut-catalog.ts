export type ShortcutKey = 'mod' | 'alt' | 'shift' | string;
export type ShortcutPlatform = 'mac' | 'windows';
export type ShortcutSequence = {
  keys: ShortcutKey[];
  platforms?: ShortcutPlatform[];
};
export type ShortcutEntry = {
  id: string;
  label: string;
  sequences: ShortcutSequence[];
  cheatsheet?: {
    label?: string;
    sequences?: ShortcutSequence[];
  };
};
export type ShortcutCategory = {
  id: string;
  title: string;
  cheatsheetColumn?: 'left' | 'right';
  shortcuts: ShortcutEntry[];
};

export const shortcutCategories: ShortcutCategory[] = [
  {
    id: 'formatting',
    title: '텍스트 서식',
    cheatsheetColumn: 'left',
    shortcuts: [
      { id: 'bold', label: '굵게', sequences: [{ keys: ['mod', 'B'] }], cheatsheet: {} },
      { id: 'italic', label: '기울임', sequences: [{ keys: ['mod', 'I'] }], cheatsheet: {} },
      { id: 'strikethrough', label: '취소선', sequences: [{ keys: ['mod', 'shift', 'S'] }], cheatsheet: {} },
      { id: 'underline', label: '밑줄', sequences: [{ keys: ['mod', 'U'] }], cheatsheet: {} },
      { id: 'clear-formatting', label: '서식 지우기', sequences: [{ keys: ['mod', '\\'] }], cheatsheet: {} },
    ],
  },
  {
    id: 'editing',
    title: '편집',
    cheatsheetColumn: 'left',
    shortcuts: [
      { id: 'undo', label: '실행 취소', sequences: [{ keys: ['mod', 'Z'] }], cheatsheet: {} },
      {
        id: 'redo',
        label: '다시 실행',
        sequences: [{ keys: ['mod', 'shift', 'Z'] }, { keys: ['mod', 'Y'], platforms: ['windows'] }],
        cheatsheet: {},
      },
      { id: 'cut', label: '잘라내기', sequences: [{ keys: ['mod', 'X'] }] },
      { id: 'copy', label: '복사', sequences: [{ keys: ['mod', 'C'] }] },
      { id: 'paste', label: '붙여넣기', sequences: [{ keys: ['mod', 'V'] }] },
      {
        id: 'paste-without-formatting',
        label: '서식 없이 붙여넣기',
        sequences: [{ keys: ['mod', 'shift', 'V'] }],
        cheatsheet: {},
      },
      { id: 'select-all', label: '전체 선택', sequences: [{ keys: ['mod', 'A'] }], cheatsheet: {} },
      {
        id: 'find-and-replace',
        label: '찾기 및 바꾸기 열기',
        sequences: [{ keys: ['mod', 'F'] }],
        cheatsheet: { label: '찾기 및 바꾸기' },
      },
      {
        id: 'previous-sentence',
        label: '이전 문장 경계로 이동',
        sequences: [{ keys: ['alt', '↑'] }],
        cheatsheet: {
          label: '문장 경계 이동',
          sequences: [{ keys: ['alt', '↑↓'] }],
        },
      },
      { id: 'next-sentence', label: '다음 문장 경계로 이동', sequences: [{ keys: ['alt', '↓'] }] },
      {
        id: 'select-previous-sentence',
        label: '이전 문장 경계까지 선택',
        sequences: [{ keys: ['shift', 'alt', '↑'] }],
      },
      {
        id: 'select-next-sentence',
        label: '다음 문장 경계까지 선택',
        sequences: [{ keys: ['shift', 'alt', '↓'] }],
      },
    ],
  },
  {
    id: 'insertion',
    title: '삽입',
    cheatsheetColumn: 'right',
    shortcuts: [
      { id: 'paragraph-break', label: '문단 나누기', sequences: [{ keys: ['Enter'] }], cheatsheet: {} },
      {
        id: 'line-break',
        label: '문단 내 줄바꿈',
        sequences: [{ keys: ['shift', 'Enter'] }],
        cheatsheet: {},
      },
      { id: 'page-break', label: '페이지 나누기', sequences: [{ keys: ['mod', 'Enter'] }], cheatsheet: {} },
      {
        id: 'insert-image-or-file',
        label: '이미지/파일 삽입',
        sequences: [{ keys: ['드래그 앤 드롭'] }, { keys: ['mod', 'V'] }],
      },
    ],
  },
  {
    id: 'menu',
    title: '메뉴',
    cheatsheetColumn: 'right',
    shortcuts: [
      { id: 'quick-search', label: '빠른 검색 열기', sequences: [{ keys: ['mod', 'K'] }], cheatsheet: {} },
      { id: 'close-ui', label: '열린 UI 닫기', sequences: [{ keys: ['Esc'] }], cheatsheet: {} },
    ],
  },
  {
    id: 'note',
    title: '노트',
    cheatsheetColumn: 'right',
    shortcuts: [
      { id: 'open-note', label: '노트 열기', sequences: [{ keys: ['mod', 'J'] }], cheatsheet: {} },
      { id: 'add-note', label: '노트 추가', sequences: [{ keys: ['mod', 'Enter'] }], cheatsheet: {} },
    ],
  },
  {
    id: 'layout',
    title: '레이아웃',
    cheatsheetColumn: 'right',
    shortcuts: [
      {
        id: 'focus-mode',
        label: '집중 모드 전환',
        sequences: [{ keys: ['mod', 'shift', 'M'] }],
        cheatsheet: {},
      },
      { id: 'toggle-prism-panel', label: 'PRISM 열기/닫기', sequences: [{ keys: ['mod', 'E'] }], cheatsheet: {} },
      { id: 'page-zoom-in', label: '페이지 확대', sequences: [{ keys: ['mod', '+'] }], cheatsheet: {} },
      { id: 'page-zoom-out', label: '페이지 축소', sequences: [{ keys: ['mod', '-'] }], cheatsheet: {} },
      { id: 'page-zoom-reset', label: '페이지 배율 100%', sequences: [{ keys: ['mod', '0'] }], cheatsheet: {} },
    ],
  },
];
