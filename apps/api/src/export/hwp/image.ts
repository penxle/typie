// spell-checker:words HWPTAG Hwpunit
import { mapFormat } from '../core/assets.ts';
import { convertPlaceholderNode } from './blocks.ts';
import { buildSectionDef, makeInlineObjectParagraph } from './paragraph.ts';
import { allocate, ctrlId, HWPTAG, makeRecord, pxToHwpunit } from './records.ts';
import { resolveParaShape } from './styles.ts';
import type { ImageAsset, NodeEntry } from '../core/types.ts';
import type { DocInfoTables } from './doc-info.ts';
import type { HwpConvertContext } from './types.ts';

/** 이미지용 gso 컨트롤 헤더 (46바이트, 글자처럼 취급) */
function buildImageCtrlHeader(width: number, height: number, instanceId: number): Uint8Array {
  const { buf, view } = allocate(46);
  view.setUint32(0, ctrlId('gso '), true);
  // like_char=1, wrap_type=2, vRelTo=3(paragraph), hRelTo=1, hAlign=4, allowOverlap=1, textFlowMethod=2
  view.setUint32(4, 0x04_0a_23_11, true);
  view.setUint32(16, width, true);
  view.setUint32(20, height, true);
  view.setUint32(24, 1, true);
  view.setUint32(36, instanceId, true);
  view.setUint16(44, 0, true); // desc_len
  return buf;
}

/** Rendering 정보: translation=identity, scale=비율, rotation=identity */
function buildPictureRenderingInfo(scaleX: number, scaleY: number): Uint8Array {
  // cnt(2) + translation_matrix(48) + scale_matrix(48) + rotation_matrix(48) = 146
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

/** SHAPE_COMPONENT for picture (표 82 + 83) */
function buildPictureShapeComponent(origWidth: number, origHeight: number, displayWidth: number, displayHeight: number): Uint8Array {
  const scaleX = displayWidth / origWidth;
  const scaleY = displayHeight / origHeight;
  const renderingInfo = buildPictureRenderingInfo(scaleX, scaleY);
  const totalSize = 8 + 42 + renderingInfo.byteLength;
  const { buf, view } = allocate(totalSize);

  view.setUint32(0, ctrlId('$pic'), true);
  view.setUint32(4, ctrlId('$pic'), true);

  let offset = 8;
  // x_offset(4) + y_offset(4) + group_level(2) + local_version(2)
  view.setUint16(offset + 10, 1, true); // local_version = 1
  offset += 12;
  view.setUint32(offset, origWidth, true); // width_org
  offset += 4;
  view.setUint32(offset, origHeight, true); // height_org
  offset += 4;
  view.setUint32(offset, displayWidth, true); // width_cur
  offset += 4;
  view.setUint32(offset, displayHeight, true); // height_cur
  offset += 4;
  // flags(4)
  view.setUint32(offset, 0x24_08_00_00, true);
  offset += 4;
  // rotation(2)
  offset += 2;
  // center_x(4) + center_y(4)
  view.setInt32(offset, Math.floor(displayWidth / 2), true);
  offset += 4;
  view.setInt32(offset, Math.floor(displayHeight / 2), true);
  offset += 4;

  buf.set(renderingInfo, offset);
  return buf;
}

/** SHAPE_COMPONENT_PICTURE 데이터 (91바이트) — vertex/crop은 원본 크기 사용 */
function buildPictureData(origWidth: number, origHeight: number, binDataId: number): Uint8Array {
  const { buf, view } = allocate(91);
  let offset = 0;

  // 테두리색(4) + 두께(4) + 속성(4) = 12
  offset += 12;

  // 이미지 크기 사각형: 원본 크기로 4개의 POINT (x,y) 쌍 인터리브
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

  // crop: (left=0, top=0, right=origWidth, bottom=origHeight) = 전체 이미지 표시
  offset += 8; // crop_left(4) + crop_top(4) = 0
  view.setInt32(offset, origWidth, true);
  offset += 4; // crop_right
  view.setInt32(offset, origHeight, true);
  offset += 4; // crop_bottom

  // inner margins (8 bytes) = 0
  offset += 8;

  // 그림 정보: 밝기(1) + 명암(1) + 효과(1) + binItemId(2, HWP 1-indexed)
  view.setUint16(offset + 3, binDataId + 1, true);

  // 투명도(1) + instanceId(4) + 추가 필드(8) = 13
  // 기본값 0

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
  const records: Uint8Array[] = [
    makeRecord(HWPTAG.CTRL_HEADER, 1, buildImageCtrlHeader(displayWidth, displayHeight, instanceId)),
    makeRecord(HWPTAG.SHAPE_COMPONENT, 2, buildPictureShapeComponent(origWidth, origHeight, displayWidth, displayHeight)),
    makeRecord(HWPTAG.SHAPE_COMPONENT_PICTURE, 3, buildPictureData(origWidth, origHeight, binDataId)),
  ];

  return records;
}

export function convertImageNode(entry: NodeEntry, ctx: HwpConvertContext, isFirst: boolean): Uint8Array[] {
  const nodeId = entry.id as string | undefined;
  if (!nodeId) return convertPlaceholderNode('[이미지]', ctx, isFirst);

  const asset = ctx.assets.get(nodeId);
  if (!asset || asset.width <= 0 || asset.height <= 0) {
    return convertPlaceholderNode('[이미지를 불러올 수 없습니다]', ctx, isFirst);
  }

  const proportion = (entry as { proportion?: number }).proportion ?? 1;
  const contentWidthPx = ctx.pageLayout.pageWidth - ctx.pageLayout.pageMarginLeft - ctx.pageLayout.pageMarginRight;
  const displayWidthPx = contentWidthPx * Math.min(proportion, 1);
  const displayHeightPx = displayWidthPx * (asset.height / asset.width);

  const origWidth = pxToHwpunit(asset.width);
  const origHeight = pxToHwpunit(asset.height);
  const displayWidth = pxToHwpunit(displayWidthPx);
  const displayHeight = pxToHwpunit(displayHeightPx);

  const ext = mapFormat(asset.format);
  const binDataId = ctx.tables.binData.intern({ extension: ext }, nodeId);

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

/** BinData 스트림 수집: cfb에 추가할 스트림 목록 반환 */
export function collectBinDataStreams(tables: DocInfoTables, assets: Map<string, ImageAsset>): Map<string, Uint8Array> {
  const streams = new Map<string, Uint8Array>();
  const binDataEntries = tables.binData.getAll();

  for (const [i, entry] of binDataEntries.entries()) {
    // binDataId는 1-based
    const id = (i + 1).toString(16).toUpperCase().padStart(4, '0');
    const streamName = `BinData/BIN${id}.${entry.extension}`;

    // assets에서 이미지 바이트 찾기 (intern이 아닌 getId로 조회하여 부작용 방지)
    for (const [nodeId, asset] of assets) {
      if (tables.binData.getId(nodeId) === i) {
        streams.set(streamName, asset.bytes);
        break;
      }
    }
  }

  return streams;
}
