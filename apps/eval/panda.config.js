import { defineConfig } from '@pandacss/dev';
import { preset } from '@typie/styled-system';

export default defineConfig({
  importMap: '@typie/styled-system',
  // 세대 모듈의 UI도 스캔해야 한다 — Panda는 정적 분석으로 CSS를 생성하므로 여기 빠지면
  // 그 파일의 css() 클래스가 아예 만들어지지 않고 스타일이 통째로 사라진다.
  include: ['./src/**/*.{js,ts,svelte}', './generations/**/*.{js,ts,svelte}', '../../packages/ui/src/**/*.{js,ts,svelte}'],

  eject: true,
  presets: [preset],

  separator: '-',
  hash: true,
  minify: true,
});
