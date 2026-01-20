export const inview = (node: HTMLElement) => {
  const isMobile = window.innerWidth < 1024;
  const observer = new IntersectionObserver(
    ([entry]) => {
      if (entry.isIntersecting) {
        node.classList.add('in-view');
        observer.disconnect();
      }
    },
    {
      threshold: isMobile ? 0.05 : 0.1,
      rootMargin: isMobile ? '0px 0px 20px 0px' : '0px 0px 50px 0px',
    },
  );
  observer.observe(node);
  return () => observer.disconnect();
};
