export type NodeEntry = Record<string, unknown> & {
  type: string;
  children?: string[];
  parent?: string;
};

export type Style =
  | { type: 'bold' }
  | { type: 'italic' }
  | { type: 'underline' }
  | { type: 'strikethrough' }
  | { type: 'font_size'; size: number }
  | { type: 'font_family'; family: string }
  | { type: 'font_weight'; weight: number }
  | { type: 'text_color'; color: string }
  | { type: 'background_color'; color: string }
  | { type: 'letter_spacing'; spacing: number };

export type Annotation = { type: 'link'; href: string } | { type: 'ruby'; text: string };

export type TextSegment = {
  text: string;
  styles: Style[];
  annotations: Annotation[];
};
