import { Menu, MenuItem, PredefinedMenuItem, Submenu } from '@tauri-apps/api/menu';
import { confirm, message } from '@tauri-apps/plugin-dialog';
import { relaunch } from '@tauri-apps/plugin-process';
import { check } from '@tauri-apps/plugin-updater';

export const ssr = false;

export const load = async () => {
  const about = await Submenu.new({
    text: '정보',
    items: [
      await MenuItem.new({
        text: '업데이트 확인',
        action: async () => {
          let update;
          try {
            update = await check();
            if (!update) {
              await message('현재 최신 버전을 이용하고 있어요.', {
                kind: 'info',
                title: '업데이트 확인',
                okLabel: '확인',
              });

              return;
            }
          } catch (err) {
            await message(`지금은 업데이트를 확인할 수 없어요.\n나중에 다시 시도해주세요.\n오류: ${err}`, {
              kind: 'error',
              title: '업데이트 확인 실패',
              okLabel: '확인',
            });

            return;
          }

          const result = await confirm('새로운 버전이 있어요.\n업데이트하시겠어요?', {
            kind: 'info',
            title: '업데이트 확인',
            okLabel: '지금 업데이트',
            cancelLabel: '나중에 하기',
          });

          if (!result) {
            return;
          }

          await update.downloadAndInstall();
          await relaunch();
        },
      }),
      await PredefinedMenuItem.new({
        item: 'Separator',
      }),
      await PredefinedMenuItem.new({
        item: 'Quit',
        text: '타이피 종료',
      }),
    ],
  });

  const edit = await Submenu.new({
    text: '편집',
    items: [
      await PredefinedMenuItem.new({
        item: 'Undo',
        text: '되돌리기',
      }),
      await PredefinedMenuItem.new({
        item: 'Redo',
        text: '다시하기',
      }),
      await PredefinedMenuItem.new({
        item: 'Separator',
      }),
      await PredefinedMenuItem.new({
        item: 'Cut',
        text: '잘라내기',
      }),
      await PredefinedMenuItem.new({
        item: 'Copy',
        text: '복사',
      }),
      await PredefinedMenuItem.new({
        item: 'Paste',
        text: '붙여넣기',
      }),
      await PredefinedMenuItem.new({
        item: 'SelectAll',
        text: '전체선택',
      }),
    ],
  });

  const menu = await Menu.new({
    items: [about, edit],
  });

  await menu.setAsAppMenu();
};
