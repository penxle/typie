export type TreeEntity = {
  id: string;
  type: 'Document' | 'Folder';
  children?: TreeEntity[];
  parentId?: string;
};
