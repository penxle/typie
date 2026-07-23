import mixpanel from 'mixpanel-browser';

let open = $state(false);
let subscribed = $state(false);

export const SubscribeModal = {
  get open() {
    return open;
  },

  show(via: string) {
    if (open) {
      return;
    }

    open = true;
    mixpanel.track('open_subscribe_modal', { via });
  },

  close() {
    open = false;
  },

  gate(via: string) {
    if (subscribed) {
      return true;
    }

    this.show(via);
    return false;
  },

  sync(value: boolean) {
    subscribed = value;
  },
};
