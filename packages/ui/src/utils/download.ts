export function downloadFromBase64(base64Data: string, filename: string, mimeType: string): void {
  const byteCharacters = atob(base64Data);
  const byteNumbers: number[] = [];
  for (let i = 0; i < byteCharacters.length; i++) {
    const codePoint = byteCharacters.codePointAt(i);
    if (codePoint !== undefined) {
      byteNumbers[i] = codePoint;
    }
  }
  const byteArray = new Uint8Array(byteNumbers);
  const blob = new Blob([byteArray], { type: mimeType });

  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.append(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}
