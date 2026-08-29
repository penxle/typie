export type TreeEntity = {
  id: string;
  type: 'Document' | 'Folder' | 'Divider';
  icon: string;
  iconColor: string;
  children?: TreeEntity[];
  parentId?: string;
};
