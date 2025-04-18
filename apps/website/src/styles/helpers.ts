export const rem = (max: number) => {
  const lists = Object.fromEntries([...Array.from({ length: max }).keys()].map((i) => [`${i + 1}px`, { value: `${(i + 1) / 16}rem` }]));

  return {
    0: { value: '0' },
    ...lists,
  };
};

export const px = (max: number) => {
  const lists = Object.fromEntries([...Array.from({ length: max }).keys()].map((i) => [`${i + 1}px`, { value: `${i + 1}px` }]));

  return {
    0: { value: '0' },
    ...lists,
  };
};
