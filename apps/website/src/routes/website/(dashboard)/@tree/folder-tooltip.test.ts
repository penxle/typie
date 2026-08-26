import { EntityVisibility } from '@typie/lib/enums';
import { mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import FolderTooltip from './FolderTooltip.svelte';
import type { Component } from 'svelte';

type TooltipProps = {
  characterCount?: number;
  documentCount?: number;
  folderCount?: number;
  loading: boolean;
  visibility: EntityVisibility;
};

const TestableFolderTooltip = FolderTooltip as Component<TooltipProps>;
let component: Record<string, unknown> | undefined;

afterEach(async () => {
  if (component) await unmount(component);
  component = undefined;
  document.body.replaceChildren();
});

it('renders supplied folder metadata without an application query context', () => {
  component = mount(TestableFolderTooltip, {
    target: document.body,
    props: {
      characterCount: 1234,
      documentCount: 3,
      folderCount: 2,
      loading: false,
      visibility: EntityVisibility.PRIVATE,
    },
  }) as Record<string, unknown>;

  expect(document.body.textContent).toContain('비공개 폴더');
  expect(document.body.textContent).toContain('2개');
  expect(document.body.textContent).toContain('3개');
  expect(document.body.textContent).toContain('총 1,234자');
});
