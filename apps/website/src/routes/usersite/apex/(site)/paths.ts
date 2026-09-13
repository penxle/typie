export const discoveryHomePath = '/';
export const discoveryLatestPath = '/latest';
export const discoveryTagsPath = '/tags';
export const discoveryTagPath = (name: string) => `/t/${encodeURIComponent(name)}`;
