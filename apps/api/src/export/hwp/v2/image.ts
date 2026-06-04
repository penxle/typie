// spell-checker:words HWPTAG Hwpunit
import { mapFormat } from '../../core/assets.ts';
import { buildSectionDef, makeInlineObjectParagraph } from '../paragraph.ts';
import { allocate, ctrlId, HWPTAG, makeRecord, pxToHwpunit } from '../records.ts';
import { resolveParaShape } from '../styles.ts';
import { convertPlaceholderNodeV2 } from './blocks.ts';
import type { ImageV2 } from '../../core/v2/types.ts';
import type { HwpConvertContext } from '../types.ts';

function buildImageCtrlHeader(width: number, height: number, instanceId: number): Uint8Array {
  const { buf, view } = allocate(46);
  view.setUint32(0, ctrlId('gso '), true);
  view.setUint32(4, 0x04_0a_23_11, true);
  view.setUint32(16, width, true);
  view.setUint32(20, height, true);
  view.setUint32(24, 1, true);
  view.setUint32(36, instanceId, true);
  view.setUint16(44, 0, true);
  return buf;
}

function buildPictureRenderingInfo(scaleX: number, scaleY: number): Uint8Array {
  const { buf, view } = allocate(146);
  view.setUint16(0, 1, true);
  const identity = [1, 0, 0, 0, 1, 0];
  const scale = [scaleX, 0, 0, 0, scaleY, 0];
  const matrices = [identity, scale, identity];
  for (let m = 0; m < 3; m++) {
    for (let i = 0; i < 6; i++) {
      view.setFloat64(2 + m * 48 + i * 8, matrices[m][i], true);
    }
  }
  return buf;
}

function buildPictureShapeComponent(origWidth: number, origHeight: number, displayWidth: number, displayHeight: number): Uint8Array {
  const scaleX = displayWidth / origWidth;
  const scaleY = displayHeight / origHeight;
  const renderingInfo = buildPictureRenderingInfo(scaleX, scaleY);
  const totalSize = 8 + 42 + renderingInfo.byteLength;
  const { buf, view } = allocate(totalSize);

  view.setUint32(0, ctrlId('$pic'), true);
  view.setUint32(4, ctrlId('$pic'), true);

  let offset = 8;
  view.setUint16(offset + 10, 1, true);
  offset += 12;
  view.setUint32(offset, origWidth, true);
  offset += 4;
  view.setUint32(offset, origHeight, true);
  offset += 4;
  view.setUint32(offset, displayWidth, true);
  offset += 4;
  view.setUint32(offset, displayHeight, true);
  offset += 4;
  view.setUint32(offset, 0x24_08_00_00, true);
  offset += 4;
  offset += 2;
  view.setInt32(offset, Math.floor(displayWidth / 2), true);
  offset += 4;
  view.setInt32(offset, Math.floor(displayHeight / 2), true);
  offset += 4;

  buf.set(renderingInfo, offset);
  return buf;
}

function buildPictureData(origWidth: number, origHeight: number, binDataId: number): Uint8Array {
  const { buf, view } = allocate(91);
  let offset = 0;

  offset += 12;

  view.setInt32(offset, 0, true);
  view.setInt32(offset + 4, 0, true);
  offset += 8;
  view.setInt32(offset, origWidth, true);
  view.setInt32(offset + 4, 0, true);
  offset += 8;
  view.setInt32(offset, origWidth, true);
  view.setInt32(offset + 4, origHeight, true);
  offset += 8;
  view.setInt32(offset, 0, true);
  view.setInt32(offset + 4, origHeight, true);
  offset += 8;

  offset += 8;
  view.setInt32(offset, origWidth, true);
  offset += 4;
  view.setInt32(offset, origHeight, true);
  offset += 4;

  offset += 8;

  view.setUint16(offset + 3, binDataId + 1, true);

  return buf;
}

function makeImageRecords(
  origWidth: number,
  origHeight: number,
  displayWidth: number,
  displayHeight: number,
  binDataId: number,
  instanceId: number,
): Uint8Array[] {
  return [
    makeRecord(HWPTAG.CTRL_HEADER, 1, buildImageCtrlHeader(displayWidth, displayHeight, instanceId)),
    makeRecord(HWPTAG.SHAPE_COMPONENT, 2, buildPictureShapeComponent(origWidth, origHeight, displayWidth, displayHeight)),
    makeRecord(HWPTAG.SHAPE_COMPONENT_PICTURE, 3, buildPictureData(origWidth, origHeight, binDataId)),
  ];
}

export function imageToRecordsV2(n: ImageV2, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const asset = n.asset;
  if (asset.width <= 0 || asset.height <= 0) {
    return convertPlaceholderNodeV2('[이미지를 불러올 수 없습니다]', ctx, isFirst);
  }

  const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
  const displayWidthPx = contentWidthPx * Math.min(n.proportion, 1);
  const displayHeightPx = displayWidthPx * (asset.height / asset.width);

  const origWidth = pxToHwpunit(asset.width);
  const origHeight = pxToHwpunit(asset.height);
  const displayWidth = pxToHwpunit(displayWidthPx);
  const displayHeight = pxToHwpunit(displayHeightPx);

  const ext = mapFormat(asset.format);
  const binDataId = ctx.tables.binData.intern({ extension: ext }, n.id);

  const instanceId = ++ctx.instanceCounter;
  const centerParaShapeId = resolveParaShape(ctx, { align: 'center' });

  return [
    ...makeInlineObjectParagraph(ctx, 0, 'gso ', {
      sectionRecords: isFirst ? buildSectionDef(ctx) : undefined,
      paraShapeId: centerParaShapeId,
    }),
    ...makeImageRecords(origWidth, origHeight, displayWidth, displayHeight, binDataId, instanceId),
  ];
}
