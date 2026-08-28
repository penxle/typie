import { Menu, nativeImage } from 'electron';
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
  openPreference: () => void;
  checkForUpdates: () => void;
  restartToUpdate: () => void;
  openWebsite: () => void;
  toggleDevTools: () => void;
  crashActiveTab: () => void;
  simulateUpdateReady: () => void;
};

const isMac = process.platform === 'darwin';

const MENU_ICONS = {
  settings: {
    1: 'iVBORw0KGgoAAAANSUhEUgAAAA0AAAANCAYAAABy6+R8AAAA6UlEQVR4nG3RDXGDMBiH8X8cIAEUDBRsDpopaOqAKdgcDAejCpY56BQ0VQASKqHPe2mgx/W5+3Hh44UDnJ5XS3rFP2ZtcthWYcIFL2hwxZJDaQ872aKDR8QZCRWOWIYGeJRsndAiomTrvgxN+IAdrLDDm6ST7ncnj280jo3VY48OQXk/Imhdn3HE4NiUJhwQlL/YlzLrJOkHDZZ36hGU3yFI+sSofPyAiIRRD0+alS+IsDxaJERYHgPqMmQ7O5TekdDiF6U/9GXIClr/k/GISHcVRtHjUMlOzspfq0OtfLMlh2fVWv/TrE03Q583DgGxfEIAAAAASUVORK5CYII=',
    2: 'iVBORw0KGgoAAAANSUhEUgAAABoAAAAaCAYAAACpSkzOAAACQklEQVR4nL3W7VHbQBRG4XcrsKggdBBRQeQOnAoiV4BdAasKUCpArgB3gFyBRQekAuQOci43iz+wsPKHM/PMMPJKuysJj4O+qP+Z6FrSHXJYHSq8aEQBY8qwxbWOe5F0gx6fFjCmUtIDdojyoqQJ5mh0oYAxNZJ+YYVSXqOPxwYLOCzDHXL0WCNDDWuKVt4Mj7AW6DFDhg4VerwVkMrwhBznWqHUcY18V+fqMEWPo4mifDf2HGpkKOStEXW+KN+J1covvMAEFaIoINVK+oEKUcPZGGuDoaJ80TamEAWkWvlFfsNWdNotaqR6RPn402rY+A0KUUAqyldhzdFo3wL3sHawJrCWqJEq5f8KVoUoCjis0f7h3qBDhldYS9SwonxhPa5g5djCWqHUvwJO6/AdczTyrT9hhwwp+/sV1hSt9jt/Ro73Ak7bwgbN0Wg/UY8rpDK8wpqi1X6iDjd4L+CwB5TybKCdYPWYIGr/8G8RdbzTHFtYjXyxbwWkovyeW3M02rfAPc61RI1UKV+wVSGKAlKtPn+97ViU78zaIep4kpQdu8UGhSgg1conqhA1XCGv1XBRfnc2KEQBqSj/sEeNDIW8NSqcy86ZwWrl5y+Qwc6JooCUfdDKX+1zNfJnd9gDSp3vGYV84qOJrAxR/vb0WCPDPayfWMOa4RHWEj1myNAhyo+9dTrRUI38G2OFUl6jj8cGCxhTKb9NtsIoL8pXP0ejCwWMyS7Y4RsO+4McPT4tYGzX8l3ksDpE+S+hiwV8SX8BAvaNGyWgtfkAAAAASUVORK5CYII=',
  },
  'circle-arrow-up': {
    1: 'iVBORw0KGgoAAAANSUhEUgAAAA0AAAANCAYAAABy6+R8AAAA6klEQVR4nGXQgXGCMBhA4ZcJxA1wAnGDdBPdwE5QnKBuUDYpnUA6QdkAnaB9aYogfXdfToE/HAn8r9BWEWiBT111bzn0qqPGBwttVQMn/RY0dtFNe6BnqgQaYKWdCEqdVSkyVfI43JLVwaXQoI16cnvgTQc15ErgS+vgEpnelNqT/69001ENuU7HNFSTByN5t3Qjkr9xp5a8YU/+3QaXSD619MC8b6X78y56ThcLDdqoZ2o5VDL7ptRZWz1prORxk3d9qA4uY50GHdQzVZJPcq1KBM2rgRd1uqpQpZNq/gpaVqhSxJNi2uDeDxj7Ni65Tfi3AAAAAElFTkSuQmCC',
    2: 'iVBORw0KGgoAAAANSUhEUgAAABoAAAAaCAYAAACpSkzOAAACKklEQVR4nL3T4XWTUBiH8f+doHQC4wSSCaQTNE7gZQLdoDcb6AQlE1gnkE4QOkHjBCUTxOftK0IawJx+6HPO796ehPQFEoLeqHMGZbjGChkW8naSWtzhJ1pMFjBVhi9IOq8k6TtanDQ1KMcvZLD2aFDLdytHId8vYLW4QoOjAl4WJd2ia42k+ZKkG3SVqDQoYFiOLawHRI2c3UQ5KkkfYC3R4LnhoAyPsP0BOV5TAxvW4j1sPxqU1F/+Eg3GKuTVGi/HFtYaSRRgZXiCtUbSeFH991ei0nhJ/Ulfog0sVpT/gz1s6FhRfsywEpXGa3GB52MCi3WHa9yj0GlRp0O6SlQ6rZb0ERvEwGLV8hfXSDouqh+ywWdYw79LVDouyW/fPYrAYj1iIekT7Oq6CvmDa20QJR1gBVTqh12hVt8KP7ATv77AYu0kvcPUoA2ivAOsAKuSD7tCrb4VbNBvLAKLVWv61r3sACtgrqSRW3eHa9yj0HwHWAFz1fKT3yAGFivKv/AWl5jrAKv77FRPyFCiCiyWvWBvWGskTXfOoCS/bXssxAUElq4kf9NaosFYhbxa4+XYwlojiQK6MuzkT3MDG/aatsixx0JcDRQwLIcdaDUoYfs55biF7dYSDZ4LeFmUf6AryW/BXDdI6itRaVDAWDlq+W20WjSo5buVo5DvGaw9CvXH/GtqkJXhK+xs/5cN+PZXi5PmBnVlWKGQtJCzdnK1/IFvMVnAm/QHpIGGGyElslkAAAAASUVORK5CYII=',
  },
};

const menuIcon = (name: keyof typeof MENU_ICONS) => {
  if (!isMac) return;
  const image = nativeImage.createEmpty();
  image.addRepresentation({ scaleFactor: 1, dataURL: `data:image/png;base64,${MENU_ICONS[name][1]}` });
  image.addRepresentation({ scaleFactor: 2, dataURL: `data:image/png;base64,${MENU_ICONS[name][2]}` });
  image.setTemplateImage(true);
  return image;
};

export const buildMenu = (
  actions: MenuActions,
  options: { devTools: boolean; updateReady: boolean; tabs: { title: string; active: boolean }[]; canReopen: boolean },
) => {
  const updateItems: MenuItemConstructorOptions[] = [
    options.updateReady
      ? { label: '재시작하여 업데이트', icon: menuIcon('circle-arrow-up'), click: actions.restartToUpdate }
      : { label: '업데이트 확인…', icon: menuIcon('circle-arrow-up'), click: actions.checkForUpdates },
  ];

  const tabItems: MenuItemConstructorOptions[] = options.tabs.slice(0, 9).map((tab, i) => ({
    type: 'radio',
    checked: tab.active,
    label: tab.title || '불러오는 중…',
    accelerator: `CmdOrCtrl+${i + 1}`,
    click: () => actions.activateTab(i),
  }));

  const hasTabs = options.tabs.length > 0;
  const hasManyTabs = options.tabs.length > 1;

  const preferenceItem: MenuItemConstructorOptions = {
    label: '설정…',
    accelerator: 'CmdOrCtrl+,',
    icon: menuIcon('settings'),
    enabled: hasTabs,
    click: actions.openPreference,
  };

  const template: MenuItemConstructorOptions[] = [
    ...(isMac
      ? [
          {
            label: '타이피',
            submenu: [
              { role: 'about' as const, label: '타이피에 관하여' },
              ...updateItems,
              { type: 'separator' as const },
              preferenceItem,
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
        { label: '새 탭', accelerator: 'CmdOrCtrl+T', enabled: hasTabs, click: actions.newTab },
        { label: '닫은 탭 다시 열기', accelerator: 'CmdOrCtrl+Shift+T', enabled: hasTabs && options.canReopen, click: actions.reopenTab },
        { type: 'separator' },
        { label: '탭 닫기', accelerator: 'CmdOrCtrl+W', enabled: hasManyTabs, click: actions.closeTab },
        { label: '창 닫기', accelerator: 'CmdOrCtrl+Shift+W', click: actions.closeWindow },
        ...(isMac
          ? []
          : [
              { type: 'separator' as const },
              preferenceItem,
              { type: 'separator' as const },
              ...updateItems,
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
        { label: '새로고침', accelerator: 'CmdOrCtrl+R', enabled: hasTabs, click: actions.reload },
        { type: 'separator' },
        { label: '뒤로', accelerator: isMac ? 'Cmd+[' : 'Alt+Left', enabled: hasTabs, click: actions.goBack },
        { label: '앞으로', accelerator: isMac ? 'Cmd+]' : 'Alt+Right', enabled: hasTabs, click: actions.goForward },
        { type: 'separator' },
        { role: 'togglefullscreen', label: '전체 화면' },
        ...(options.devTools
          ? [
              { type: 'separator' as const },
              { label: '개발자 도구', accelerator: isMac ? 'Alt+Cmd+I' : 'Ctrl+Shift+I', enabled: hasTabs, click: actions.toggleDevTools },
              { label: '탭 크래시(개발용)', enabled: hasTabs, click: actions.crashActiveTab },
              { label: '업데이트 알약 표시(개발용)', click: actions.simulateUpdateReady },
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
        { label: '다음 탭', accelerator: 'Ctrl+Tab', enabled: hasManyTabs, click: actions.nextTab },
        { label: '이전 탭', accelerator: 'Ctrl+Shift+Tab', enabled: hasManyTabs, click: actions.prevTab },
        ...(isMac
          ? [
              { label: '다음 탭', accelerator: 'Cmd+Alt+Right', enabled: hasManyTabs, click: actions.nextTab, visible: false },
              { label: '이전 탭', accelerator: 'Cmd+Alt+Left', enabled: hasManyTabs, click: actions.prevTab, visible: false },
            ]
          : []),
        ...(tabItems.length > 0 ? [{ type: 'separator' as const }, ...tabItems] : []),
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
