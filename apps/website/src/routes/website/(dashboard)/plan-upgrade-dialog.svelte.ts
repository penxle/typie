type PlanUpgradeDialogOptions = {
  title?: string;
  message: string;
};

let current = $state<PlanUpgradeDialogOptions | null>(null);

export const PlanUpgradeDialog = {
  get current() {
    return current;
  },

  show(options: PlanUpgradeDialogOptions) {
    current = options;
  },

  close() {
    current = null;
  },
};
