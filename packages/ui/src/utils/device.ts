export const isMobileDevice = () => {
  // cspell:disable-next-line
  return /mobi|webos|android|iphone|ipad|ipod|blackberry|iemobile|opera mini/i.test(navigator.userAgent);
};
