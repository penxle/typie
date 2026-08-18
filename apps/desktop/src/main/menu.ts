import { Menu } from 'electron';
import type { MenuItemConstructorOptions } from 'electron';

export type MenuActions = {
  newTab: () => void;
  closeTab: () => void;
  closeWindow: () => void;
  reopenTab: () => void;
  reload: () => void;
  goBack: () => void;
  goForward: () => void;
  nextTab: () => void;
  prevTab: () => void;
  activateTab: (index: number) => void;
  activateLastTab: () => void;
  checkForUpdates: () => void;
  openWebsite: () => void;
  toggleDevTools: () => void;
  crashActiveTab: () => void;
};

const isMac = process.platform === 'darwin';

export const buildMenu = (actions: MenuActions, options: { devTools: boolean }) => {
  const tabItems: MenuItemConstructorOptions[] = [
    ...Array.from({ length: 8 }, (_, i) => ({
      label: `탭 ${i + 1}`,
      accelerator: `CmdOrCtrl+${i + 1}`,
      click: () => actions.activateTab(i),
    })),
    { label: '마지막 탭', accelerator: 'CmdOrCtrl+9', click: actions.activateLastTab },
  ];

  const template: MenuItemConstructorOptions[] = [
    ...(isMac
      ? [
          {
            label: '타이피',
            submenu: [
              { role: 'about' as const, label: '타이피에 관하여' },
              { label: '업데이트 확인…', click: actions.checkForUpdates },
              { type: 'separator' as const },
              { role: 'services' as const, label: '서비스' },
              { type: 'separator' as const },
              { role: 'hide' as const, label: '타이피 숨기기' },
              { role: 'hideOthers' as const, label: '다른 앱 숨기기' },
              { role: 'unhide' as const, label: '모두 표시' },
              { type: 'separator' as const },
              { role: 'quit' as const, label: '타이피 종료' },
            ],
          },
        ]
      : []),
    {
      label: '파일',
      submenu: [
        { label: '새 탭', accelerator: 'CmdOrCtrl+T', click: actions.newTab },
        { label: '닫은 탭 다시 열기', accelerator: 'CmdOrCtrl+Shift+T', click: actions.reopenTab },
        { type: 'separator' },
        { label: '탭 닫기', accelerator: 'CmdOrCtrl+W', click: actions.closeTab },
        { label: '창 닫기', accelerator: 'CmdOrCtrl+Shift+W', click: actions.closeWindow },
        ...(isMac
          ? []
          : [
              { type: 'separator' as const },
              { label: '업데이트 확인…', click: actions.checkForUpdates },
              { role: 'quit' as const, label: '종료' },
            ]),
      ],
    },
    {
      label: '편집',
      submenu: [
        { role: 'undo', label: '실행 취소' },
        { role: 'redo', label: '다시 실행' },
        { type: 'separator' },
        { role: 'cut', label: '잘라내기' },
        { role: 'copy', label: '복사' },
        { role: 'paste', label: '붙여넣기' },
        { role: 'selectAll', label: '전체 선택' },
      ],
    },
    {
      label: '보기',
      submenu: [
        { label: '새로고침', accelerator: 'CmdOrCtrl+R', click: actions.reload },
        { type: 'separator' },
        { label: '뒤로', accelerator: isMac ? 'Cmd+[' : 'Alt+Left', click: actions.goBack },
        { label: '앞으로', accelerator: isMac ? 'Cmd+]' : 'Alt+Right', click: actions.goForward },
        { type: 'separator' },
        { role: 'togglefullscreen', label: '전체 화면' },
        ...(options.devTools
          ? [
              { type: 'separator' as const },
              { label: '개발자 도구', accelerator: isMac ? 'Alt+Cmd+I' : 'Ctrl+Shift+I', click: actions.toggleDevTools },
              { label: '탭 크래시(개발용)', click: actions.crashActiveTab },
            ]
          : []),
      ],
    },
    {
      label: '창',
      submenu: [
        { role: 'minimize', label: '최소화' },
        { role: 'zoom', label: '확대/축소' },
        { type: 'separator' },
        { label: '다음 탭', accelerator: 'Ctrl+Tab', click: actions.nextTab },
        { label: '이전 탭', accelerator: 'Ctrl+Shift+Tab', click: actions.prevTab },
        ...(isMac
          ? [
              { label: '다음 탭', accelerator: 'Cmd+Alt+Right', click: actions.nextTab, visible: false },
              { label: '이전 탭', accelerator: 'Cmd+Alt+Left', click: actions.prevTab, visible: false },
            ]
          : []),
        { type: 'separator' },
        ...tabItems,
        ...(isMac ? [{ type: 'separator' as const }, { role: 'front' as const, label: '모두 앞으로 가져오기' }] : []),
      ],
    },
    {
      role: 'help',
      label: '도움말',
      submenu: [{ label: '타이피 웹사이트', click: actions.openWebsite }],
    },
  ];

  return Menu.buildFromTemplate(template);
};
