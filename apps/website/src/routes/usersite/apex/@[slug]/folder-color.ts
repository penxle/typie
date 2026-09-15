import { hashText } from '@typie/lib/title-page';
import { css } from '@typie/styled-system/css';
import type { SystemStyleObject } from '@typie/styled-system/types';

export const FOLDER_COLORS = ['red', 'orange', 'yellow', 'green', 'blue', 'purple'] as const;

export type FolderColor = (typeof FOLDER_COLORS)[number];

export const folderColor = (id: string): FolderColor => FOLDER_COLORS[hashText(id) % FOLDER_COLORS.length];

const folderCoverStyles: Record<FolderColor, SystemStyleObject> = {
  red: css.raw({ backgroundColor: 'palette.red/20', color: 'palette.red' }),
  orange: css.raw({ backgroundColor: 'palette.orange/20', color: 'palette.orange' }),
  yellow: css.raw({ backgroundColor: 'palette.yellow/20', color: 'palette.yellow' }),
  green: css.raw({ backgroundColor: 'palette.green/20', color: 'palette.green' }),
  blue: css.raw({ backgroundColor: 'palette.blue/20', color: 'palette.blue' }),
  purple: css.raw({ backgroundColor: 'palette.purple/20', color: 'palette.purple' }),
};

export const folderCoverStyle = (id: string): SystemStyleObject => folderCoverStyles[folderColor(id)];
