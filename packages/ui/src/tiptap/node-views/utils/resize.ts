import { clamp } from '../../../utils/number';
import { mmToPx } from '../../../utils/unit';
import type { PageLayout } from '../../../utils';

export type ResizeData = {
  x: number;
  width: number;
  proportion: number;
  reverse: boolean;
};

export type ResizeConstraints = {
  proportion: number;
  minProportion: number;
};

export function getMaxContentHeight(pageLayout: PageLayout | undefined): number | undefined {
  if (!pageLayout) {
    return undefined;
  }
  return mmToPx(pageLayout.height - pageLayout.marginTop - pageLayout.marginBottom);
}

export function calculateConstrainedProportion(
  proposedProportion: number,
  containerEl: HTMLElement | undefined,
  pageLayout: PageLayout | undefined,
): ResizeConstraints {
  if (!pageLayout || !containerEl) {
    return { proportion: proposedProportion, minProportion: 0.1 };
  }

  const maxContentHeight = getMaxContentHeight(pageLayout);
  if (!maxContentHeight) {
    return { proportion: proposedProportion, minProportion: 0.1 };
  }

  const mediaElement = containerEl.querySelector('img, iframe') as HTMLElement;
  if (!mediaElement) {
    return { proportion: proposedProportion, minProportion: 0.1 };
  }

  const parentWidth = containerEl.parentElement?.clientWidth || 0;
  const proposedWidth = parentWidth * proposedProportion;

  const currentRect = mediaElement.getBoundingClientRect();
  const aspectRatio = currentRect.width / currentRect.height;
  const proposedHeight = proposedWidth / aspectRatio;

  // NOTE: 이미지가 maxContentHeight를 넘지 않기 위한 최대 너비
  const maxWidthForHeight = maxContentHeight * aspectRatio;
  const minProportionForHeight = maxWidthForHeight / parentWidth;

  // NOTE: 더 작은 값을 최소 proportion으로 사용
  const minProportion = Math.min(0.1, minProportionForHeight);

  let constrainedProportion = proposedProportion;
  if (proposedHeight > maxContentHeight) {
    constrainedProportion = maxWidthForHeight / parentWidth;
  }

  return { proportion: constrainedProportion, minProportion };
}

export function checkAndAdjustProportion(
  currentProportion: number,
  containerEl: HTMLElement | undefined,
  pageLayout: PageLayout | undefined,
  updateAttributes: (attrs: { proportion: number }) => void,
): number {
  const { proportion: constrainedProportion, minProportion } = calculateConstrainedProportion(currentProportion, containerEl, pageLayout);

  const clampedProportion = clamp(constrainedProportion, minProportion, 1);

  if (clampedProportion !== currentProportion) {
    updateAttributes({ proportion: clampedProportion });
    return clampedProportion;
  }

  return currentProportion;
}

export function calculateResizeProportion(
  dx: number,
  initialData: ResizeData,
  containerEl: HTMLElement | undefined,
  pageLayout: PageLayout | undefined,
): number {
  const proposedProportion = ((initialData.width + dx * 2) / initialData.width) * initialData.proportion;

  const { proportion: constrainedProportion, minProportion } = calculateConstrainedProportion(proposedProportion, containerEl, pageLayout);

  return clamp(constrainedProportion, minProportion, 1);
}
