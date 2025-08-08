export const comma = (value: number) => {
  return value.toLocaleString('en-US');
};

export const clamp = (value: number, min: number, max: number) => {
  return Math.min(Math.max(value, min), max);
};

export const closest = (value: number, array: number[]) => {
  if (Number.isNaN(value) || array.length === 0) {
    return null;
  }

  let r = array[0];

  for (let i = 1; i < array.length; i++) {
    if (Math.abs(value - array[i]) < Math.abs(value - r)) {
      r = array[i];
    }
  }

  return r;
};

export const formatBytes = (bytes: number) => {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let index = 0;

  while (bytes >= 1000 && index < units.length - 1) {
    bytes /= 1000;
    index++;
  }

  return `${Math.floor(bytes)} ${units[index]}`;
};
