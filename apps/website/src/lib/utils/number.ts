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
