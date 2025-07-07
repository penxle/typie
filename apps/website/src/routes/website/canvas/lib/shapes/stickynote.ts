import Konva from 'konva';
import { renderRoughDrawable, roughGenerator } from '../rough';
import type { TypedContentfulShapeConfig } from './types';

type TypedStickyNoteShapeConfig = TypedContentfulShapeConfig & {
  width: number;
  height: number;
};

type TypedStickyNoteConfig = TypedStickyNoteShapeConfig & {
  text: string;
  fontSize: number;
  fontFamily: string;
};

class StickyNoteShape extends Konva.Shape<TypedStickyNoteShapeConfig> {
  _sceneFunc(context: Konva.Context) {
    const { width: w, height: h, roughness, backgroundColor, backgroundStyle, seed } = this.attrs;
    const foldSize = Math.min(30, Math.min(w, h) * 0.15);

    const bodyPath = `
      M 0 0
      L 0 ${h}
      L ${w - foldSize} ${h}
      L ${w} ${h - foldSize}
      L ${w} 0
      Z
    `;

    const bodyDrawable = roughGenerator.path(bodyPath.trim(), {
      roughness: roughness === 'rough' ? 1 : 0,
      bowing: 0.2,
      stroke: 'none',
      seed,
      fill: backgroundColor || '#fef3c7',
      fillStyle: backgroundStyle === 'none' ? undefined : backgroundStyle,
      fillWeight: 1,
      hachureGap: 6,
      preserveVertices: true,
    });

    context.save();
    context.shadowColor = 'rgba(0, 0, 0, 0.1)';
    context.shadowBlur = 4;
    context.shadowOffsetX = 2;
    context.shadowOffsetY = 2;
    renderRoughDrawable(context, bodyDrawable);
    context.restore();

    const borderDrawable = roughGenerator.path(bodyPath.trim(), {
      roughness: roughness === 'rough' ? 1 : 0,
      bowing: 0.2,
      stroke: 'rgba(0, 0, 0, 0.1)',
      strokeWidth: 1,
      seed,
      fill: 'none',
    });
    renderRoughDrawable(context, borderDrawable);

    const foldPath = `
      M ${w - foldSize} ${h}
      L ${w - foldSize} ${h - foldSize}
      L ${w} ${h - foldSize}
      Z
    `;

    if (roughness === 'rough') {
      const foldDrawable = roughGenerator.path(foldPath.trim(), {
        roughness: 0.5,
        bowing: 0.1,
        stroke: 'rgba(0, 0, 0, 0.15)',
        strokeWidth: 1,
        seed: seed + 1,
        fill: 'rgba(0, 0, 0, 0.05)',
        fillStyle: 'solid',
      });
      renderRoughDrawable(context, foldDrawable);
    } else {
      context.save();
      context.fillStyle = 'rgba(0, 0, 0, 0.05)';
      context.strokeStyle = 'rgba(0, 0, 0, 0.15)';
      context.lineWidth = 1;
      context.beginPath();
      context.moveTo(w - foldSize, h);
      context.lineTo(w - foldSize, h - foldSize);
      context.lineTo(w, h - foldSize);
      context.closePath();
      context.fill();
      context.stroke();
      context.restore();
    }
  }

  _hitFunc(context: Konva.Context) {
    const { width: w, height: h } = this.attrs;
    const foldSize = Math.min(30, Math.min(w, h) * 0.15);

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
      roughness: config.roughness,
      backgroundColor: config.backgroundColor,
      backgroundStyle: config.backgroundStyle,
      seed: config.seed,
    });

    this.text = new Konva.Text({
      x: 0,
      y: 1,
      text: config.text,
      fontSize: config.fontSize,
      fontFamily: config.fontFamily,
      fill: 'black',
      width: config.width,
      padding: 15,
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

  override setAttrs(config: Partial<TypedStickyNoteConfig>) {
    super.setAttrs({
      x: config.x ?? this.attrs.x,
      y: config.y ?? this.attrs.y,
      width: config.width ?? this.attrs.width,
      height: config.height ?? this.attrs.height,
    });

    Object.assign(this.attrs, config);

    this.shape?.setAttrs({
      width: config.width ?? this.attrs.width,
      height: config.height ?? this.attrs.height,
      roughness: config.roughness ?? this.attrs.roughness,
      backgroundColor: config.backgroundColor ?? this.attrs.backgroundColor,
      backgroundStyle: config.backgroundStyle ?? this.attrs.backgroundStyle,
    });

    this.text?.setAttrs({
      width: config.width ?? this.attrs.width,
      text: config.text ?? this.attrs.text,
      fontSize: config.fontSize ?? this.attrs.fontSize,
      fontFamily: config.fontFamily ?? this.attrs.fontFamily,
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
      padding: `${15 * scale}px`,
      color: 'black',
      fontSize: `${this.attrs.fontSize * scale}px`,
      fontFamily: this.attrs.fontFamily,
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
      this.#textarea.style.fontSize = `${this.attrs.fontSize * scale}px`;
      this.#textarea.style.padding = `${15 * scale}px`;
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
