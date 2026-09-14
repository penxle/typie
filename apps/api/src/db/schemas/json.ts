export type CouponCondition = Record<string, unknown>;

export type PlanRules = {
  maxTotalCharacterCount: number;
  maxTotalBlobSize: number;
};

export type DocLayoutMode =
  | {
      type: 'paginated';
      pageWidth: number;
      pageHeight: number;
      pageMarginTop: number;
      pageMarginBottom: number;
      pageMarginLeft: number;
      pageMarginRight: number;
    }
  | { type: 'continuous'; maxWidth: number };
