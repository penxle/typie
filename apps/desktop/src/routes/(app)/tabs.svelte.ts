import { nanoid } from 'nanoid';
import type { Component } from 'svelte';

type TabProps = { tabId: string; [key: string]: unknown };
type TabComponent<T extends TabProps = TabProps> = Component<T>;

export type Tab<T extends TabProps = TabProps> = {
  id: string;
  title?: string;
  active: boolean;
  component: TabComponent<T>;
  props: Omit<T, 'tabId'>;
};

class TabState {
  tabs = $state<Tab[]>([]);

  get active() {
    return this.tabs.find((tab) => tab.active);
  }

  add<T extends TabProps>(component: TabComponent<T>, props: Omit<T, 'tabId'>) {
    this.tabs.forEach((tab) => (tab.active = false));
    this.tabs.push({
      id: nanoid(),
      active: true,
      component: component as TabComponent,
      props,
    });
  }

  remove(id: string) {
    const index = this.tabs.findIndex((tab) => tab.id === id);
    if (index === -1) return;

    if (this.tabs.length === 1) {
      return;
    }

    const wasActive = this.tabs[index].active;
    this.tabs.splice(index, 1);

    if (wasActive) {
      const newActiveIndex = Math.min(index, this.tabs.length - 1);
      this.tabs[newActiveIndex].active = true;
    }
  }

  switch(id: string) {
    const tab = this.tabs.find((t) => t.id === id);
    if (!tab) return;

    this.tabs.forEach((t) => (t.active = false));
    tab.active = true;
  }

  navigate<T extends TabProps>(id: string, component: TabComponent<T>, props: Omit<T, 'tabId'>) {
    const tab = this.tabs.find((t) => t.id === id);
    if (tab) {
      tab.component = component as TabComponent;
      tab.props = props;
    }
  }

  setTitle(id: string, title: string) {
    const tab = this.tabs.find((t) => t.id === id);
    if (tab) {
      tab.title = title;
    }
  }
}

export const tabState = new TabState();
