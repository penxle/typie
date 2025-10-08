export type PlanRules = {
  maxTotalCharacterCount: number;
  maxTotalBlobSize: number;
};

export type CanvasShape = {
  type: string;
  attrs: Record<string, unknown>;
};

export type PageLayout = {
  width: number;
  height: number;
  marginTop: number;
  marginBottom: number;
  marginLeft: number;
  marginRight: number;
};
