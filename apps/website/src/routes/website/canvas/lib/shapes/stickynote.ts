import Konva from 'konva';
import { clamp } from '$lib/utils';
import { MIN_SIZE } from '../const';
import { renderRoughDrawable, roughGenerator } from '../rough';
import { values } from '../values';
import { TypedShape } from './shape';
import type { BackgroundColor } from '../values';
import type { TypedShapeConfig } from './types';

type TypedStickyNoteConfig = TypedShapeConfig & {
  width: number;
  height: number;
  backgroundColor: BackgroundColor;
  seed: number;
  text: string;
};

const FONT_SIZE = 16;

export class TypedStickyNote extends TypedShape<TypedStickyNoteConfig> {
  #wrapper?: HTMLDivElement;
  #textarea?: HTMLTextAreaElement;

  #isEditing = false;
  #boundUpdateTextareaPosition: () => void;

  constructor(config: TypedStickyNoteConfig) {
    super(config);

    this.on('dblclick', () => this.#startEditing());
    this.#boundUpdateTextareaPosition = () => this.#updateTextareaPosition();
  }

  get effectiveFoldSize() {
    const { width, height } = this.attrs;
    return Math.min(MIN_SIZE * 2, Math.min(width, height) * 0.2);
  }

  getVerticalOffset() {
    const fontSize = FONT_SIZE;
    const lineHeightPx = fontSize * 1.4;

    const unitsPerEm = 2816;
    const ascender = 2728;
    const descender = -680;

    const fontSizeEm = fontSize / unitsPerEm;
    const lineGap = (lineHeightPx - fontSizeEm * ascender + fontSizeEm * descender) / 2;

    return fontSizeEm * ascender + lineGap;
  }

  override renderView(context: Konva.Context) {
    const { width: w, height: h, backgroundColor, seed, text } = this.attrs;

    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const bgColorHex = values.backgroundColor.find((c) => c.value === backgroundColor)!.color;
    const roughness = clamp(Math.min(w, h) / (MIN_SIZE * 10) - 1, 0.5, 2.5);
    const foldSize = this.effectiveFoldSize;

    const notePath = `M 0 0 L 0 ${h} L ${w - foldSize} ${h} L ${w} ${h - foldSize} L ${w} 0 Z`;

    const ctx = context._context;
    ctx.save();
    const note = new Path2D(notePath);
    ctx.fillStyle = bgColorHex;
    ctx.shadowColor = 'rgba(0, 0, 0, 0.1)';
    ctx.shadowOffsetX = 2;
    ctx.shadowOffsetY = 2;
    ctx.shadowBlur = 4;
    ctx.fill(note);
    ctx.restore();

    const border = roughGenerator.path(notePath, {
      roughness,
      bowing: 1,
      stroke: 'rgba(0, 0, 0, 0.1)',
      strokeWidth: 1,
      seed,
    });

    renderRoughDrawable(context, border);

    const foldPath = `M ${w - foldSize} ${h} L ${w - foldSize} ${h - foldSize} L ${w} ${h - foldSize} Z`;

    const fold = roughGenerator.path(foldPath, {
      roughness,
      bowing: 1,
      stroke: 'rgba(0, 0, 0, 0.15)',
      strokeWidth: 1,
      fill: 'rgba(0, 0, 0, 0.05)',
      fillStyle: 'solid',
      seed,
    });

    renderRoughDrawable(context, fold);

    if (text && !this.#isEditing) {
      ctx.save();

      ctx.font = `${FONT_SIZE}px Interop`;
      ctx.fillStyle = 'black';
      ctx.textAlign = 'left';
      ctx.textBaseline = 'alphabetic';

      const padding = 20;
      const lineHeight = FONT_SIZE * 1.4;
      const maxWidth = w - padding * 2;

      const textX = padding;

      const paragraphs = text.split('\n');
      const lines: string[] = [];

      for (const paragraph of paragraphs) {
        if (paragraph === '') {
          lines.push('');
          continue;
        }

        let currentLine = '';

        for (const char of paragraph) {
          const testLine = currentLine + char;
          const metrics = ctx.measureText(testLine);

          if (metrics.width > maxWidth && currentLine) {
            lines.push(currentLine);
            currentLine = char;
          } else {
            currentLine = testLine;
          }
        }

        if (currentLine) {
          lines.push(currentLine);
        }
      }

      const verticalOffset = this.getVerticalOffset();

      for (const [i, line] of lines.entries()) {
        const y = padding + i * lineHeight + verticalOffset;
        if (y > h - padding) break;
        ctx.fillText(line, textX, y);
      }

      ctx.restore();
    }
  }

  override renderHitTest(context: Konva.Context) {
    const { width: w, height: h } = this.attrs;
    const foldSize = this.effectiveFoldSize;

    context.beginPath();
    context.moveTo(0, 0);
    context.lineTo(0, h);
    context.lineTo(w - foldSize, h);
    context.lineTo(w, h - foldSize);
    context.lineTo(w, 0);
    context.closePath();
    context.fillStrokeShape(this);
  }

  #createTextarea() {
    if (this.#textarea) return;

    const stage = this.getStage();
    if (!stage) return;

    const rect = this.getClientRect();
    const scale = stage.scaleX();
    const { text } = this.attrs;

    this.#wrapper = document.createElement('div');
    Object.assign(this.#wrapper.style, {
      position: 'absolute',
      left: '0',
      top: '0',
      width: `${rect.width}px`,
      height: `${rect.height}px`,
      transform: `translate(${rect.x}px, ${rect.y}px)`,
      display: 'flex',
      alignItems: 'flex-start',
      justifyContent: 'flex-start',
      padding: `${20 * scale}px`,
      zIndex: '1000',
      pointerEvents: 'none',
    });

    this.#textarea = document.createElement('textarea');
    this.#textarea.value = text;
    this.#textarea.rows = 1;

    Object.assign(this.#textarea.style, {
      width: '100%',
      height: 'auto',
      minHeight: '0',
      maxHeight: '100%',
      color: 'black',
      fontSize: `${FONT_SIZE * scale}px`,
      fontFamily: 'Interop',
      textAlign: 'left',
      lineHeight: '1.4',
      letterSpacing: '-0.03em',
      whiteSpace: 'pre-wrap',
      wordBreak: 'break-all',
      overflow: 'auto',
      resize: 'none',
      pointerEvents: 'auto',
    });

    this.#textarea.addEventListener('blur', () => this.#finalizeEditing());
    this.#textarea.addEventListener('keydown', (e) => {
      e.stopPropagation();

      if (e.key === 'Escape' || (e.key === 'Enter' && (e.ctrlKey || e.metaKey))) {
        e.preventDefault();
        this.#finalizeEditing();
      }
    });

    this.#textarea.addEventListener('input', () => this.#adjustTextareaHeight());

    this.#wrapper.append(this.#textarea);
    document.body.append(this.#wrapper);

    this.#textarea.focus();
    this.#textarea.select();

    this.#adjustTextareaHeight();
  }

  #adjustTextareaHeight() {
    if (!this.#textarea || !this.#wrapper) return;

    const stage = this.getStage();
    if (!stage) return;

    const scale = stage.scaleX();
    const padding = 20 * scale;

    this.#textarea.style.height = 'auto';
    const scrollHeight = this.#textarea.scrollHeight;
    this.#textarea.style.height = `${scrollHeight}px`;
    const totalHeight = scrollHeight + padding * 2;
    const rectHeight = this.attrs.height * scale;

    if (totalHeight > rectHeight) {
      const newRectHeight = totalHeight / scale;
      this.setAttrs({ height: newRectHeight });
      this.#wrapper.style.height = `${totalHeight}px`;
    } else {
      this.#wrapper.style.height = `${rectHeight}px`;
    }
  }

  #updateTextareaPosition() {
    const stage = this.getStage();
    if (!stage) return;

    requestAnimationFrame(() => {
      if (!this.#textarea || !this.#wrapper) return;

      const rect = this.getClientRect();
      const scale = stage.scaleX();

      this.#wrapper.style.transform = `translate(${rect.x}px, ${rect.y}px)`;
      this.#wrapper.style.width = `${rect.width}px`;
      this.#wrapper.style.padding = `${20 * scale}px`;

      this.#textarea.style.fontSize = `${FONT_SIZE * scale}px`;

      this.#adjustTextareaHeight();
    });
  }

  #startEditing() {
    if (this.#isEditing) return;

    this.#isEditing = true;
    this.#createTextarea();

    const stage = this.getStage();
    if (stage) {
      stage.on('xChange yChange scaleXChange scaleYChange', this.#boundUpdateTextareaPosition);
      this.on('xChange yChange', this.#boundUpdateTextareaPosition);
    }

    this.getLayer()?.batchDraw();
  }

  #finalizeEditing() {
    if (!this.#isEditing || !this.#textarea) return;

    this.#isEditing = false;

    this.setAttrs({ text: this.#textarea.value });
    this.#destroyTextarea();

    this.getLayer()?.batchDraw();
  }

  #destroyTextarea() {
    if (!this.#textarea) return;

    this.#textarea.remove();
    this.#textarea = undefined;

    if (this.#wrapper) {
      this.#wrapper.remove();
      this.#wrapper = undefined;
    }

    const stage = this.getStage();
    if (stage) {
      stage.off('xChange yChange scaleXChange scaleYChange', this.#boundUpdateTextareaPosition);
      this.off('xChange yChange', this.#boundUpdateTextareaPosition);
    }
  }

  override destroy() {
    this.#destroyTextarea();
    return super.destroy();
  }
}
