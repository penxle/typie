import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { renderAsync } from '@resvg/resvg-js';
import ky from 'ky';
import satori from 'satori';
import sharp from 'sharp';

const loadFonts = async <T extends string>(names: T[]) => {
  const load = async (name: string) => {
    const filePath = path.join('/tmp/fonts', `${name}.ttf`);
    try {
      return await readFile(filePath);
    } catch {
      const url = `https://cdn.typie.net/fonts/ttf/${name}.ttf`;
      const resp = await ky.get(url).arrayBuffer();
      await mkdir(path.dirname(filePath), { recursive: true });
      await writeFile(filePath, new Uint8Array(resp));
      return resp;
    }
  };
  return Object.fromEntries(await Promise.all(names.map(async (name) => [name, await load(name)]))) as Record<T, ArrayBuffer>;
};

const fonts = await loadFonts(['Paperlogy-4Regular', 'Paperlogy-7Bold']);

type PreviewTheme = 'light' | 'dark';

const themeColors = {
  light: {
    background: '#FFFFFF', // surface.default (white)
    title: '#09090c', // text.default (gray.950)
    subtitle: '#70717b', // text.faint (gray.500)
    divider: '#d3d4dd', // border.strong (gray.300)
  },
  dark: {
    background: '#121316', // surface.default (dark.gray.900) -- mobile
    title: '#f1f1f7', // text.default (dark.gray.50)
    subtitle: '#a3a4a9', // text.faint (dark.gray.300)
    divider: '#414246', // border.strong (dark.gray.600)
  },
};

const RENDER_WIDTH = 1200;
const RENDER_HEIGHT = 1600;

type PreviewLayoutParams = {
  title: string;
  subtitle: string | null;
  bodySvg: string;
  theme: PreviewTheme;
};

export async function renderPreviewImage(params: PreviewLayoutParams, outputWidth: number): Promise<Uint8Array> {
  const { title, subtitle, bodySvg, theme } = params;
  const c = themeColors[theme];

  // 본문 SVG를 먼저 PNG로 래스터화 (SVG 안의 <image> data URI 중첩 문제 방지)
  const bodyPng = await renderAsync(bodySvg, {
    font: { loadSystemFonts: false },
    imageRendering: 0,
    shapeRendering: 2,
    textRendering: 1,
    fitTo: { mode: 'width', value: RENDER_WIDTH },
  });
  const bodyBase64 = Uint8Array.from(bodyPng.asPng()).toBase64();
  const bodyDataUri = `data:image/png;base64,${bodyBase64}`;

  const node = (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        width: `${RENDER_WIDTH}px`,
        height: `${RENDER_HEIGHT}px`,
        backgroundColor: c.background,
        fontFamily: 'Paperlogy',
        padding: '160px',
      }}
    >
      <div
        style={{
          display: 'block',
          fontSize: '96px',
          fontWeight: 700,
          color: c.title,
          lineHeight: '1.3',
          lineClamp: 2,
          wordBreak: 'break-all',
        }}
      >
        {title}
      </div>

      {subtitle && (
        <div
          style={{
            display: 'block',
            fontSize: '72px',
            fontWeight: 400,
            color: c.subtitle,
            marginTop: '24px',
            lineClamp: 1,
          }}
        >
          {subtitle}
        </div>
      )}

      <div
        style={{
          display: 'flex',
          width: '100%',
          height: '2px',
          backgroundColor: c.divider,
          marginTop: '48px',
          marginBottom: '48px',
        }}
      />

      <div
        style={{
          display: 'flex',
          flex: 1,
          overflow: 'hidden',
        }}
      >
        <img
          src={bodyDataUri}
          style={{
            width: '100%',
            objectFit: 'cover',
            objectPosition: 'top',
          }}
        />
      </div>
    </div>
  );

  const svg = await satori(node, {
    width: RENDER_WIDTH,
    height: RENDER_HEIGHT,
    fonts: [
      { name: 'Paperlogy', data: fonts['Paperlogy-4Regular'], weight: 400 },
      { name: 'Paperlogy', data: fonts['Paperlogy-7Bold'], weight: 700 },
    ],
  });

  const png = await renderAsync(svg, {
    font: { loadSystemFonts: false },
    imageRendering: 0,
    shapeRendering: 2,
    textRendering: 1,
  });

  const webp = await sharp(Uint8Array.from(png.asPng())).resize(outputWidth).webp({ quality: 80 }).toBuffer();

  return new Uint8Array(webp);
}
