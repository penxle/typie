import { EntityState } from '@typie/lib/enums';

export type EntityTreeRevealRequest = {
  targetEntityId: string;
  ancestorFolderIds: string[];
  rename: boolean;
};

let current = $state<EntityTreeRevealRequest>();

export const entityTreeRevealState = {
  get current() {
    return current;
  },

  set(request: EntityTreeRevealRequest | undefined) {
    current = request;
  },

  consume(handled: EntityTreeRevealRequest) {
    current = consumeEntityTreeRevealRequest(current, handled);
  },
};

export const resolveActiveTreeAncestorIds = (state: EntityState, ancestorIds: string[]): string[] =>
  state === EntityState.ACTIVE ? ancestorIds : [];

export const createEntityTreeRevealRequest = (
  targetEntityId: string,
  ancestorFolderIds: string[],
  rename: boolean,
): EntityTreeRevealRequest => ({ targetEntityId, ancestorFolderIds: [...ancestorFolderIds], rename });

export const shouldOpenEntityTreeFolder = (folderEntityId: string): boolean => current?.ancestorFolderIds.includes(folderEntityId) ?? false;

export const shouldConsumeDocumentRevealRequest = (
  request: EntityTreeRevealRequest | undefined,
  documentEntityId: string,
  active: boolean,
  navigationIdle: boolean,
): boolean => active && navigationIdle && request?.targetEntityId === documentEntityId;

export const consumeEntityTreeRevealRequest = (
  currentRequest: EntityTreeRevealRequest | undefined,
  handled: EntityTreeRevealRequest,
): EntityTreeRevealRequest | undefined => (currentRequest === handled ? undefined : currentRequest);
