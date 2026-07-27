import { TableCode } from '#/db/schemas/codes.ts';
import { decodeDbId } from '#/db/schemas/id.ts';
import type { BlobLeaseKind } from '#/utils/blob-lease.ts';
import type { AssetNonceItem, AssetStateEntry, ReadyAssetPayload } from './protocol.ts';

// 이 모듈은 db·redis·pubsub을 임포트하지 않는 leaf다 — sync asset 경로의 분류/조립 규칙을 IO 없이
// 단위 테스트할 수 있게 하기 위한 것이므로, 여기에 IO 의존을 들이지 말 것.

export type AssetLeaseSummary = { kind: BlobLeaseKind; name: string; size: number };

// 문서가 참조하는 네 종류를 모두 sync로 해석한다. 길이 검증(isValidAssetId)이 아니라 TableCode
// 접두사로만 분류한다 — 과거 다른 길이로 발급된 id도 완결된 asset이면 ready로 해석돼야 하기 때문이다.
export type AssetResolveKind = BlobLeaseKind | 'embed' | 'archived';

export const assetKindOf = (assetId: string): AssetResolveKind | null => {
  const code = decodeDbId(assetId);
  if (code === TableCode.IMAGES) return 'image';
  if (code === TableCode.FILES) return 'file';
  if (code === TableCode.EMBEDS) return 'embed';
  if (code === TableCode.DOCUMENT_ARCHIVED_NODES) return 'archived';
  return null;
};

// lease는 클라이언트가 바이트를 쥐고 있는 동안만 존재하므로 image/file에만 있다. heartbeat·failed와
// lease MGET은 이 판정을 써야 한다(embed·archived id가 섞여 들어와도 lease 경로로 새지 않게).
export const leaseKindOf = (assetId: string): BlobLeaseKind | null => {
  const kind = assetKindOf(assetId);
  return kind === 'image' || kind === 'file' ? kind : null;
};

// assetIds는 lease MGET 순서와 1:1로 맞춰야 하므로 그룹 결과에서 함께 만든다(image/file만).
export const groupAssetIds = (
  ids: string[],
): { imageIds: string[]; fileIds: string[]; embedIds: string[]; archivedIds: string[]; assetIds: string[] } => {
  const imageIds = ids.filter((id) => assetKindOf(id) === 'image');
  const fileIds = ids.filter((id) => assetKindOf(id) === 'file');
  const embedIds = ids.filter((id) => assetKindOf(id) === 'embed');
  const archivedIds = ids.filter((id) => assetKindOf(id) === 'archived');
  return { imageIds, fileIds, embedIds, archivedIds, assetIds: [...imageIds, ...fileIds] };
};

export const toLeaseItems = (items: AssetNonceItem[]): { assetId: string; nonce: string }[] =>
  items.filter((item) => leaseKindOf(item.id) !== null).map((item) => ({ assetId: item.id, nonce: item.nonce }));

export const toLeaseMap = <T>(assetIds: string[], leases: (T | null)[]): Map<string, T> => {
  const map = new Map<string, T>();
  for (const [index, assetId] of assetIds.entries()) {
    const lease = leases[index];
    if (lease) map.set(assetId, lease);
  }
  return map;
};

// completed row가 lease를 이긴다 — finalize가 끝난 asset은 lease 잔여물과 무관하게 ready다.
// 아는 TableCode가 아니면 ready/lease가 주어져도 missing으로 고정한다(분류 규칙의 단일 강제 지점).
export const buildAssetStateEntries = (
  ids: string[],
  ready: ReadonlyMap<string, ReadyAssetPayload>,
  leases: ReadonlyMap<string, AssetLeaseSummary>,
): AssetStateEntry[] =>
  ids.map((id): AssetStateEntry => {
    const kind = assetKindOf(id);
    if (kind === null) return { id, state: 'missing' };

    const asset = ready.get(id);
    if (asset) return { id, state: 'ready', asset };

    // embed·archived는 lease를 갖지 않는다 — 행이 없으면 곧바로 missing이다.
    const lease = kind === 'embed' || kind === 'archived' ? undefined : leases.get(id);
    if (lease) return { id, state: 'pending', meta: { kind: lease.kind, name: lease.name, size: lease.size } };

    return { id, state: 'missing' };
  });
