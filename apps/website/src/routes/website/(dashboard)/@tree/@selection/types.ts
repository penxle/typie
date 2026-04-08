export type TreeEntity = {
  id: string;
  type: 'Document' | 'Folder';
  icon: string;
  iconColor: string;
  children?: TreeEntity[];
  parentId?: string;
};
