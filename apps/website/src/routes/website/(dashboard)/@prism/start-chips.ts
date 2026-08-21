import { reviewStartChip } from './review/start-chip.ts';
import type { Component } from 'svelte';

export type StartChip = { label: string; insert: string; icon: Component };

export const startChips: StartChip[] = [reviewStartChip];
