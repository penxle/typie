import Konva from 'konva';
import { clamp } from '$lib/utils';
import { MIN_SIZE } from '../const';
import { renderRoughDrawable, roughGenerator } from '../rough';
import { values } from '../values';
import { TypedShape } from './shape';
import type { FontFamily, FontSize } from '../values';
import type { TypedContentfulShapeConfig } from './types';

const FONT_METRICS = {
  sans: {
    unitsPerEm: 900,
    ascender: 880,
    descender: -180,
    lineHeight: 1.4,
  },
  handwriting: {
    unitsPerEm: 1000,
    ascender: 906,
    descender: -339,
    lineHeight: 1.4,
  },
};

type TypedEllipseConfig = TypedContentfulShapeConfig & {
  radiusX: number;
  radiusY: number;
  text: string;
  fontSize: FontSize;
  fontFamily: FontFamily;
  textAlign: 'left' | 'center' | 'right';
};

export class TypedEllipse extends TypedShape<TypedEllipseConfig> {
  #wrapper?: HTMLDivElement;
  #textarea?: HTMLTextAreaElement;

  #isEditing = false;
  #boundUpdateTextareaPosition: () => void;

  constructor(config: TypedEllipseConfig) {
    super(config);

    this.on('dblclick', () => this.#startEditing());
    this.#boundUpdateTextareaPosition = () => this.#updateTextareaPosition();
  }

  get effectiveFontSize() {
    const { fontSize } = this.attrs;
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    return values.fontSize.find((f) => f.value === fontSize)!.size;
  }

  get effectiveFontFamily() {
    const { fontFamily } = this.attrs;
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    return values.fontFamily.find((f) => f.value === fontFamily)!.fontFamily;
  }
  get effectiveRoughness() {
    const { radiusX, radiusY, roughness } = this.attrs;

    if (roughness === 'none') {
      return 0;
    }

    const min = Math.min(radiusX * 2, radiusY * 2);

    return clamp(min / MIN_SIZE - 1, 0.2, 2);
  }

  getVerticalOffset() {
    const { fontFamily } = this.attrs;
    const fontSize = this.effectiveFontSize;
    const lineHeightPx = fontSize * 1.4;

    const metrics = FONT_METRICS[fontFamily] || FONT_METRICS.sans;
    const { unitsPerEm, ascender, descender } = metrics;

    const fontSizeEm = fontSize / unitsPerEm;
    const lineGap = (lineHeightPx - fontSizeEm * ascender + fontSizeEm * descender) / 2;

    return fontSizeEm * ascender + lineGap;
  }

  override renderView(context: Konva.Context) {
    const { radiusX, radiusY, backgroundColor, backgroundStyle, seed, text, textAlign } = this.attrs;

    const bgColorHex = values.backgroundColor.find((c) => c.value === backgroundColor)?.hex;

    const drawable = roughGenerator.ellipse(0, 0, radiusX * 2, radiusY * 2, {
      roughness: this.effectiveRoughness,
      bowing: 1,
      stroke: 'black',
      strokeWidth: 4,
      seed,
      fill: backgroundStyle === 'none' ? undefined : bgColorHex,
      fillStyle: backgroundStyle === 'none' ? undefined : backgroundStyle,
      fillWeight: 1,
      hachureGap: 8,
      curveFitting: 1,
    });

    renderRoughDrawable(context, drawable);

    if (text && !this.#isEditing) {
      const ctx = context._context;
      ctx.save();

      ctx.font = `${this.effectiveFontSize}px ${this.effectiveFontFamily}`;
      ctx.fillStyle = 'black';
      ctx.textAlign = textAlign;
      ctx.textBaseline = 'alphabetic';

      const padding = 20;
      const lineHeight = this.effectiveFontSize * 1.4;

      const maxWidth = radiusX * 2 - padding * 2;

      let textX = -radiusX + padding;
      if (textAlign === 'center') {
        textX = 0;
      } else if (textAlign === 'right') {
        textX = radiusX - padding;
      }

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
      const totalTextHeight = lines.length * lineHeight;
      const availableHeight = radiusY * 2 - padding * 2;

      const startY = -radiusY + padding + (availableHeight - totalTextHeight) / 2 + verticalOffset;

      for (const [i, line] of lines.entries()) {
        const y = startY + i * lineHeight;
        if (y > radiusY - padding) break;
        ctx.fillText(line, textX, y);
      }

      ctx.restore();
    }
  }

  override renderHitTest(context: Konva.Context) {
    const { radiusX, radiusY } = this.attrs;

    context.beginPath();
    context.ellipse(0, 0, radiusX, radiusY, 0, 0, Math.PI * 2);
    context.fillStrokeShape(this);
  }

  getWidth() {
    return this.attrs.radiusX * 2;
  }

  getHeight() {
    return this.attrs.radiusY * 2;
  }

  #createTextarea() {
    if (this.#textarea) return;

    const stage = this.getStage();
    if (!stage) return;

    const rect = this.getClientRect();
    const scale = stage.scaleX();
    const { text, textAlign } = this.attrs;

    this.#wrapper = document.createElement('div');
    Object.assign(this.#wrapper.style, {
      position: 'absolute',
      left: '0',
      top: '0',
      width: `${rect.width}px`,
      height: `${rect.height}px`,
      transform: `translate(${rect.x}px, ${rect.y}px)`,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
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
      fontSize: `${this.effectiveFontSize * scale}px`,
      fontFamily: this.effectiveFontFamily,
      textAlign,
      lineHeight: '1.4',
      letterSpacing: '-0.01em',
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
    const rectHeight = this.attrs.radiusY * 2 * scale;

    if (totalHeight > rectHeight) {
      const newRadiusY = totalHeight / scale / 2;
      this.setAttrs({ radiusY: newRadiusY });
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

      this.#textarea.style.fontSize = `${this.effectiveFontSize * scale}px`;

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

TypedEllipse.prototype._centroid = true;
TypedEllipse.prototype._attrsAffectingSize = ['radiusX', 'radiusY'];
