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
            var observer = new ResizeObserver((entries) => {
              for (var entry of entries) {
                parent.postMessage({ type: 'resize', height: entry.contentRect.height }, '*');
              }
            });

            observer.observe(document.body);
          });
        </script>
      </body>
    </html>
  `;
};
