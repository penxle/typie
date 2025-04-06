export const transform = (content: string) => {
  return `
    <html>
      <head>
        <style>html, body { margin: 0 }</style>
      </head>
      <body>
        ${content}
        <script>
          window.addEventListener('DOMContentLoaded', () => {
            const observer = new ResizeObserver((entries) => {
              for (const entry of entries) {
                parent.postMessage({ type: 'resize', height: entry.borderBoxSize[0].blockSize }, '*');
              }
            });

            observer.observe(document.documentElement, { box: 'border-box' });
          });
        </script>
      </body>
    </html>
  `;
};
