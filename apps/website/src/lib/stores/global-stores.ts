import { writable } from 'svelte/store';
import { persisted } from './persisted';

export const expandSidebar = persisted<boolean | null>('expandSidebar');
export const sidebarPopoverVisible = writable<boolean>(false);
