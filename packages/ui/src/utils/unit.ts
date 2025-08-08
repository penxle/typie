let mmPx: number | undefined;

export const mmToPx = (mm: number) => {
  if (mmPx === undefined) {
    const element = document.createElement('div');
    element.style.width = '1mm';
    element.style.position = 'absolute';
    element.style.visibility = 'hidden';
    document.body.append(element);
    mmPx = element.offsetWidth;
    element.remove();
  }

  return mm * mmPx;
};
