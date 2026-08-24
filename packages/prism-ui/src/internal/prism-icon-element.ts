import {
  PRISM_ICON_GEOMETRY,
  PRISM_ICON_IDLE_POSE,
  PRISM_ICON_IDLE_VISIBLE_EDGE_INDICES,
  projectPrismIconPose,
} from './prism-icon-morph.ts';

const SVG_NAMESPACE = 'http://www.w3.org/2000/svg';
const VIEW_BOX_SIZE = 24;

const projectedVertices = projectPrismIconPose(PRISM_ICON_IDLE_POSE, VIEW_BOX_SIZE);
const pathData = PRISM_ICON_IDLE_VISIBLE_EDGE_INDICES.map((edgeIndex) => {
  const edge = PRISM_ICON_GEOMETRY.edges[edgeIndex];
  const start = projectedVertices[edge[0]];
  const end = projectedVertices[edge[1]];
  return `M${start[0].toFixed(4)} ${start[1].toFixed(4)}L${end[0].toFixed(4)} ${end[1].toFixed(4)}`;
}).join(' ');

export function createPrismIconElement(document: Document, size: number): SVGSVGElement {
  const svg = document.createElementNS(SVG_NAMESPACE, 'svg');
  const path = document.createElementNS(SVG_NAMESPACE, 'path');
  svg.setAttribute('aria-hidden', 'true');
  svg.setAttribute('fill', 'none');
  svg.setAttribute('height', String(size));
  svg.setAttribute('viewBox', `0 0 ${VIEW_BOX_SIZE} ${VIEW_BOX_SIZE}`);
  svg.setAttribute('width', String(size));
  path.setAttribute('d', pathData);
  path.setAttribute('fill', 'none');
  path.setAttribute('stroke', 'currentColor');
  path.setAttribute('stroke-linecap', 'round');
  path.setAttribute('stroke-linejoin', 'round');
  path.setAttribute('stroke-width', '2');
  svg.append(path);
  return svg;
}
