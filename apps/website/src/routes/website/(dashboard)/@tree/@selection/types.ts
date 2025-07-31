export type TreeEntity = {
  id: string;
  type: 'Post' | 'Canvas' | 'Folder';
  children?: TreeEntity[];
  parentId?: string;
};
