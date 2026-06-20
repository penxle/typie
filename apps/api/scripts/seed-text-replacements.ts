#!/usr/bin/env node

import { db, TextReplacements } from '#/db/index.ts';
import { generateFractionalOrder } from '#/utils/index.ts';

// spell-checker:disable
const presets = [
  { id: 'TXR0DASH', match: '--', substitute: '\u{2014}', note: '하이픈 두 개를 줄표(\u{2014})로' },
  { id: 'TXR0ELLIPSIS', match: '...', substitute: '\u{2026}', note: '마침표 세 개를 말줄임표(\u{2026})로' },
  { id: 'TXR0SQUOTEOPEN', match: "(?<!\u{2018}[^\u{2019}]*)'", substitute: '\u{2018}', regex: true, note: '스마트 따옴표 (여는 홑따옴표)' },
  {
    id: 'TXR0SQUOTECLOSE',
    match: "(?<=\u{2018}[^\u{2019}]*)'",
    substitute: '\u{2019}',
    regex: true,
    note: '스마트 따옴표 (닫는 홑따옴표)',
  },
  { id: 'TXR0DQUOTEOPEN', match: '(?<!\u{201C}[^\u{201D}]*)"', substitute: '\u{201C}', regex: true, note: '스마트 따옴표 (여는 쌍따옴표)' },
  {
    id: 'TXR0DQUOTECLOSE',
    match: '(?<=\u{201C}[^\u{201D}]*)"',
    substitute: '\u{201D}',
    regex: true,
    note: '스마트 따옴표 (닫는 쌍따옴표)',
  },
];
// spell-checker:enable

let lastOrder: string | undefined;

for (const preset of presets) {
  const order = generateFractionalOrder({ lower: lastOrder, upper: undefined });
  lastOrder = order;

  await db
    .insert(TextReplacements)
    .values({ ...preset, preset: true, order })
    .onConflictDoUpdate({
      target: TextReplacements.id,
      set: {
        match: preset.match,
        substitute: preset.substitute,
        regex: preset.regex ?? false,
        note: preset.note,
        order,
      },
    });
}

process.exit(0);
