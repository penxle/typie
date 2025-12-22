export type TreeEntity = {
  id: string;
  type: 'Post' | 'Document' | 'Folder';
  children?: TreeEntity[];
  parentId?: string;
};
