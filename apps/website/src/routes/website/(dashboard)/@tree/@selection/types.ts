export type TreeEntity = {
  id: string;
  type: 'Post' | 'Canvas' | 'Document' | 'Folder';
  children?: TreeEntity[];
  parentId?: string;
};
