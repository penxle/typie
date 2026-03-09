import mime from 'mime';

export function extFromFormat(format: string): string {
  return mime.getExtension(format) ?? 'png';
}
