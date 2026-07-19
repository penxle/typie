import { EntityVisibility } from '@typie/lib/enums';

export type DocumentsOptionPolicyInput = {
  availability?: string | null;
  visibility?: string | null;
  password?: string | null;
  thumbnailId?: string | null;
  contentRating?: string | null;
  allowReaction?: boolean | null;
  protectContent?: boolean | null;
};

export const isPrivateVisibilityOnlyInput = (input: DocumentsOptionPolicyInput): boolean =>
  input.visibility === EntityVisibility.PRIVATE &&
  input.availability == null &&
  input.password == null &&
  input.thumbnailId == null &&
  input.contentRating == null &&
  input.allowReaction == null &&
  input.protectContent == null;
