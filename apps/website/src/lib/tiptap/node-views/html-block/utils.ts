export const transform = (content: string) => {
  const parser = new DOMParser();
  const doc = parser.parseFromString(content, 'text/html');

  const head = getHTML(doc.head);
  const body = getHTML(doc.body);

  return `
    <html>
      <head>
        ${head}
      </head>
      <body>
        ${body}
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

const getHTML = (element: HTMLElement) => {
  const textarea = document.createElement('textarea');
  textarea.innerHTML = element.innerHTML;
  return textarea.value;
};
