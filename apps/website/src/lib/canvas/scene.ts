import Konva from 'konva';

export class Scene {
  #stage: Konva.Stage;
  #layer: Konva.Layer;

  constructor(stage: Konva.Stage) {
    this.#stage = stage;
    this.#layer = new Konva.Layer();
    this.#stage.add(this.#layer);
  }

  get layer() {
    return this.#layer;
  }
}
