import { ENTITY_ICON_COLORS, ENTITY_ICON_NAMES } from '@typie/lib/catalogs';
import { describe, expect, it } from 'vitest';
import { entityIconColors, entityIconMap, entityIcons } from './entity-icons.ts';

describe('엔티티 아이콘 카탈로그 대조', () => {
  it('카탈로그의 모든 아이콘 이름에 컴포넌트가 붙어 있다', () => {
    expect(entityIcons.length).toBe(ENTITY_ICON_NAMES.length);
    expect(entityIconMap.size).toBe(ENTITY_ICON_NAMES.length);

    for (const name of ENTITY_ICON_NAMES) {
      expect(entityIconMap.get(name), name).toBeDefined();
    }
  });

  it('카탈로그의 모든 아이콘 색에 이름표와 색이 붙어 있다', () => {
    expect(entityIconColors.length).toBe(ENTITY_ICON_COLORS.length);

    for (const value of ENTITY_ICON_COLORS) {
      const option = entityIconColors.find((color) => color.value === value);
      expect(option?.label, value).toBeDefined();
      expect(option?.color, value).toBeDefined();
    }
  });
});
