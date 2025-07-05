import Konva from 'konva';

export class Scene {
  #stage: Konva.Stage;
  #layer: Konva.Layer;

  #transformer: Konva.Transformer;

  constructor(stage: Konva.Stage) {
    this.#stage = stage;
    this.#layer = new Konva.Layer();
    this.#stage.add(this.#layer);

    this.#transformer = new Konva.Transformer();
    this.#layer.add(this.#transformer);
  }

  get layer() {
    return this.#layer;
  }

  get transformer() {
    return this.#transformer;
  }
}
