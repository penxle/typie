export type MaintenanceConfig = {
  enabled: boolean;
  title: string;
  message: string;
  until: string | null;
  platforms: string[];
};

export type Bootstrap = {
  version: number;
  updatedAt: string;
  maintenance: MaintenanceConfig;
};
