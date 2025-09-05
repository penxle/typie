import { TextRun } from 'docx';

export function createHardBreak(): TextRun {
  return new TextRun({
    text: '',
    break: 1,
  });
}
