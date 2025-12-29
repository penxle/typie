export type CouponCondition = Record<string, unknown>;

export type PlanRules = {
  maxTotalCharacterCount: number;
  maxTotalBlobSize: number;
};

export type PageLayout = {
  width: number;
  height: number;
  marginTop: number;
  marginBottom: number;
  marginLeft: number;
  marginRight: number;
};
