import Konva from 'konva';
import Cookies from 'universal-cookie';
import { GRID_SIZE } from './const';

export class Environment {
  #stage: Konva.Stage;
  #layer: Konva.Layer;

  constructor(stage: Konva.Stage) {
    this.#stage = stage;
    this.#layer = new Konva.Layer();
    this.#stage.add(this.#layer);

    this.update();
  }

  update() {
    const stageScale = this.#stage.scaleX();
    const stagePos = this.#stage.position();
    const stageWidth = this.#stage.width();
    const stageHeight = this.#stage.height();

    let gridSize = GRID_SIZE;
    let visualSpacing = gridSize * stageScale;

    while (visualSpacing < 15) {
      gridSize *= 2;
      visualSpacing = gridSize * stageScale;
    }

    while (visualSpacing > 80 && gridSize > GRID_SIZE) {
      gridSize /= 2;
      visualSpacing = gridSize * stageScale;
    }

    const theme = new Cookies().get('typie-th');

    const grid = new Konva.Shape({
      sceneFunc: (context) => {
        const isDarkMode = theme === 'dark';
        context.strokeStyle = isDarkMode ? 'rgba(255, 255, 255, 0.05)' : 'rgba(0, 0, 0, 0.05)';
        context.lineWidth = 1 / stageScale;

        const viewLeft = -stagePos.x / stageScale;
        const viewTop = -stagePos.y / stageScale;
        const viewRight = viewLeft + stageWidth / stageScale;
        const viewBottom = viewTop + stageHeight / stageScale;

        const startX = Math.floor(viewLeft / gridSize) * gridSize;
        const endX = Math.ceil(viewRight / gridSize) * gridSize;
        const startY = Math.floor(viewTop / gridSize) * gridSize;
        const endY = Math.ceil(viewBottom / gridSize) * gridSize;

        for (let x = startX; x <= endX; x += gridSize) {
          context.beginPath();
          context.moveTo(x, startY);
          context.lineTo(x, endY);
          context.stroke();
        }

        for (let y = startY; y <= endY; y += gridSize) {
          context.beginPath();
          context.moveTo(startX, y);
          context.lineTo(endX, y);
          context.stroke();
        }
      },
    });

    this.#layer.destroyChildren();

    this.#layer.add(grid);

    this.#layer.batchDraw();
  }
}
