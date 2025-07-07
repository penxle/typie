import Konva from 'konva';
import { clamp } from '$lib/utils';
import { MIN_SIZE } from '../const';
import { renderRoughDrawable, roughGenerator } from '../rough';
import { values } from '../values';
import type { TypedShapeConfig } from './types';

type TypedStickyNoteShapeConfig = TypedShapeConfig & {
  width: number;
  height: number;
  backgroundColor: string;
  seed: number;
};

type TypedStickyNoteConfig = TypedStickyNoteShapeConfig & {
  text: string;
  fontSize: number;
  fontFamily: string;
};

class StickyNoteShape extends Konva.Shape<TypedStickyNoteShapeConfig> {
  get effectiveFoldSize() {
    const { width, height } = this.attrs;
    return Math.min(MIN_SIZE * 2, Math.min(width, height) * 0.2);
  }

  _sceneFunc(context: Konva.Context) {
    const { width: w, height: h, backgroundColor, seed } = this.attrs;

    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const bgColorHex = values.backgroundColor.find((c) => c.value === backgroundColor)!.hex;
    const roughness = clamp(Math.min(w, h) / (MIN_SIZE * 10) - 1, 0.5, 2.5);
    const foldSize = this.effectiveFoldSize;

    const notePath = `M 0 0 L 0 ${h} L ${w - foldSize} ${h} L ${w} ${h - foldSize} L ${w} 0 Z`;

    context.save();
    const note = new Path2D(notePath);
    context.fillStyle = bgColorHex;
    context.shadowColor = 'rgba(0, 0, 0, 0.1)';
    context.shadowOffsetX = 2;
    context.shadowOffsetY = 2;
    context.shadowBlur = 4;
    context.fill(note);
    context.restore();

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
  }

  _hitFunc(context: Konva.Context) {
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
}

export class TypedStickyNote extends Konva.Group {
  declare attrs: TypedStickyNoteConfig;
  shape: StickyNoteShape;
  text: Konva.Text;

  #textarea?: HTMLTextAreaElement;
  #isEditing = false;
  #boundUpdateTextareaPosition: () => void;

  constructor(config: TypedStickyNoteConfig) {
    super({
      x: config.x,
      y: config.y,
      width: config.width,
      height: config.height,
    });

    this.attrs = config;

    this.shape = new StickyNoteShape({
      x: 0,
      y: 0,
      width: config.width,
      height: config.height,
      backgroundColor: config.backgroundColor,
      seed: config.seed,
    });

    this.text = new Konva.Text({
      x: 0,
      y: 1,
      text: config.text,
      fontSize: this.effectiveFontSize,
      fontFamily: this.effectiveFontFamily,
      fill: 'black',
      width: config.width,
      padding: 20,
      lineHeight: 1.2,
      letterSpacing: 0.01,
      wrap: 'char',
      listening: false,
    });

    this.add(this.shape);
    this.add(this.text);

    this.on('dblclick', () => this.#startEditing());

    this.#boundUpdateTextareaPosition = () => this.#updateTextareaPosition();
  }

  get effectiveFontFamily() {
    const { fontFamily } = this.attrs;
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    return values.fontFamily.find((f) => f.value === fontFamily)!.fontFamily;
  }

  get effectiveFontSize() {
    const { fontFamily, fontSize } = this.attrs;

    if (fontFamily === 'handwriting') {
      return fontSize * 1.2;
    }

    return fontSize;
  }

  override setAttrs(config: Partial<TypedStickyNoteConfig>) {
    super.setAttrs({
      x: config.x ?? this.attrs.x,
      y: config.y ?? this.attrs.y,
      width: config.width ?? this.attrs.width,
      height: config.height ?? this.attrs.height,
    });

    Object.assign(this.attrs, config);

    this.fire('attrchange', { target: this }, true);

    this.shape?.setAttrs({
      width: config.width ?? this.attrs.width,
      height: config.height ?? this.attrs.height,
      backgroundColor: config.backgroundColor ?? this.attrs.backgroundColor,
    });

    this.text?.setAttrs({
      width: config.width ?? this.attrs.width,
      text: config.text ?? this.attrs.text,
      fontSize: this.effectiveFontSize,
      fontFamily: this.effectiveFontFamily,
    });

    return this;
  }

  #createTextarea() {
    if (this.#textarea) return;

    const stage = this.getStage();
    if (!stage) return;

    const rect = this.getClientRect();
    const scale = stage.scaleX();

    this.#textarea = document.createElement('textarea');
    this.#textarea.value = this.attrs.text;

    Object.assign(this.#textarea.style, {
      position: 'absolute',
      left: '0',
      top: '0',
      width: `${rect.width}px`,
      height: `${rect.height}px`,
      padding: `${20 * scale}px`,
      color: 'black',
      fontSize: `${this.effectiveFontSize * scale}px`,
      fontFamily: this.effectiveFontFamily,
      lineHeight: '1.2',
      letterSpacing: '0',
      whiteSpace: 'pre-wrap',
      wordBreak: 'break-all',
      overflow: 'hidden',
      zIndex: '1000',
      resize: 'none',
      transform: `translate(${rect.x}px, ${rect.y}px)`,
    });

    this.#textarea.addEventListener('blur', () => this.#finalizeEditing());
    this.#textarea.addEventListener('keydown', (e) => {
      e.stopPropagation();

      if (e.key === 'Escape' || (e.key === 'Enter' && (e.ctrlKey || e.metaKey))) {
        e.preventDefault();
        this.#finalizeEditing();
      }
    });

    document.body.append(this.#textarea);

    this.#textarea.focus();
  }

  #updateTextareaPosition() {
    const stage = this.getStage();
    if (!stage) return;

    requestAnimationFrame(() => {
      if (!this.#textarea) return;

      const rect = this.getClientRect();
      const scale = stage.scaleX();

      this.#textarea.style.transform = `translate(${rect.x}px, ${rect.y}px)`;
      this.#textarea.style.left = '0';
      this.#textarea.style.top = '0';
      this.#textarea.style.width = `${rect.width}px`;
      this.#textarea.style.height = `${rect.height}px`;
      this.#textarea.style.fontSize = `${this.effectiveFontSize * scale}px`;
      this.#textarea.style.padding = `${20 * scale}px`;
    });
  }

  #startEditing() {
    if (this.#isEditing) return;

    this.#isEditing = true;

    this.text.hide();
    this.#createTextarea();

    const stage = this.getStage();
    if (stage) {
      stage.on('xChange yChange scaleXChange scaleYChange', this.#boundUpdateTextareaPosition);
      this.on('xChange yChange', this.#boundUpdateTextareaPosition);
    }
  }

  #finalizeEditing() {
    if (!this.#isEditing || !this.#textarea) return;

    this.#isEditing = false;

    this.setAttrs({ text: this.#textarea.value });

    this.text.show();
    this.#destroyTextarea();
  }

  #destroyTextarea() {
    if (!this.#textarea) return;

    this.#textarea.remove();
    this.#textarea = undefined;

    const stage = this.getStage();
    if (stage) {
      stage.off('xChange yChange scaleXChange scaleYChange', this.#boundUpdateTextareaPosition);
      this.off('xChange yChange', this.#boundUpdateTextareaPosition);
    }
  }

  override destroy() {
    this.#destroyTextarea();
    super.destroy();
    return this;
  }
}
