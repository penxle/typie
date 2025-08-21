import fs from 'node:fs/promises';
import path from 'node:path';
import { renderAsync } from '@resvg/resvg-js';
import ky from 'ky';
import { Lazy } from './lazy';

const colors = {
  primary: ['#3B82F6', '#10B981', '#F59E0B', '#EF4444', '#8B5CF6', '#EC4899', '#06B6D4', '#84CC16'],
  gray: '#E5E7EB',
  text: '#1F2937',
  background: '#FFFFFF',
};

const CHART_WIDTH = 2000;
const CHART_HEIGHT = 1000;
const PADDING = 100;
const TITLE_Y = 60;
const TITLE_FONT_SIZE = 36;
const LEGEND_HEIGHT = 60;
const CHART_INNER_HEIGHT = CHART_HEIGHT - PADDING * 2 - 120;

export type ChartData = {
  labels: string[];
  datasets: { label: string; data: number[] }[];
};

const loadFonts = async <T extends string>(names: T[]) => {
  const load = async (name: string) => {
    const filePath = path.join('/tmp/fonts', `${name}.otf`);

    try {
      return await fs.readFile(filePath);
    } catch {
      const url = `https://cdn.typie.net/fonts/otf/${name}.otf`;
      const resp = await ky.get(url).arrayBuffer();

      await fs.mkdir(path.dirname(filePath), { recursive: true });
      await fs.writeFile(filePath, Buffer.from(resp));

      return resp;
    }
  };

  return Object.fromEntries(await Promise.all(names.map(async (name) => [name, await load(name)]))) as Record<T, ArrayBuffer>;
};

const lazyFonts = new Lazy(() => loadFonts(['Interop-Regular', 'Interop-Bold']));

const createSvgBase = (title: string) => {
  let svg = `<svg width="${CHART_WIDTH}" height="${CHART_HEIGHT}" xmlns="http://www.w3.org/2000/svg">`;
  svg += `<rect width="${CHART_WIDTH}" height="${CHART_HEIGHT}" fill="${colors.background}"/>`;
  svg += `<text x="${CHART_WIDTH / 2}" y="${TITLE_Y}" font-size="${TITLE_FONT_SIZE}" font-weight="bold" text-anchor="middle" fill="${colors.text}">${title}</text>`;
  return svg;
};

const drawAxes = (chartWidth: number, chartHeight: number) => {
  let svg = '';
  svg += `<line x1="${PADDING}" y1="${PADDING}" x2="${PADDING}" y2="${PADDING + chartHeight}" stroke="${colors.gray}" stroke-width="3"/>`;
  svg += `<line x1="${PADDING}" y1="${PADDING + chartHeight}" x2="${PADDING + chartWidth}" y2="${PADDING + chartHeight}" stroke="${colors.gray}" stroke-width="3"/>`;
  return svg;
};

const drawGridLines = (chartWidth: number, chartHeight: number, maxValue: number, isDashed = false) => {
  let svg = '';
  for (let i = 0; i <= 4; i++) {
    const y = PADDING + (chartHeight * i) / 4;
    const value = Math.round(maxValue * (1 - i / 4));
    const dashArray = isDashed ? ' stroke-dasharray="5,5"' : '';
    svg += `<line x1="${PADDING}" y1="${y}" x2="${PADDING + chartWidth}" y2="${y}" stroke="${colors.gray}" stroke-width="2" opacity="0.3"${dashArray}/>`;
    svg += `<text x="${PADDING - 15}" y="${y + 8}" font-size="20" text-anchor="end" fill="${colors.gray}">${value}</text>`;
  }
  return svg;
};

const drawLegend = (datasets: ChartData['datasets']) => {
  if (datasets.length <= 1) return '';

  let svg = '';
  const legendY = CHART_HEIGHT - LEGEND_HEIGHT;
  const legendItemWidth = 200;
  const startX = (CHART_WIDTH - datasets.length * legendItemWidth) / 2;

  datasets.forEach((dataset, i) => {
    const x = startX + i * legendItemWidth;
    svg += `<rect x="${x}" y="${legendY}" width="16" height="16" fill="${colors.primary[i % colors.primary.length]}" rx="2"/>`;
    svg += `<text x="${x + 25}" y="${legendY + 12}" font-size="22" fill="${colors.text}">${dataset.label}</text>`;
  });

  return svg;
};

export const generateChart = async (title: string, type: 'bar' | 'line' | 'pie', data: ChartData): Promise<Buffer> => {
  let svgContent = '';

  switch (type) {
    case 'bar': {
      svgContent = generateBarChartSVG(title, data);
      break;
    }
    case 'line': {
      svgContent = generateLineChartSVG(title, data);
      break;
    }
    case 'pie': {
      svgContent = generatePieChartSVG(title, data);
      break;
    }
  }

  await lazyFonts.get();

  const resvg = await renderAsync(svgContent, {
    background: colors.background,
    font: {
      defaultFontFamily: 'Interop-Regular',
      defaultFontSize: 20,
      loadSystemFonts: false,
      fontFiles: ['/tmp/fonts/Interop-Regular.otf', '/tmp/fonts/Interop-Bold.otf'],
    },
    imageRendering: 0,
    shapeRendering: 2,
    textRendering: 1,
    fitTo: {
      mode: 'width',
      value: CHART_WIDTH,
    },
  });

  return Buffer.from(resvg.asPng());
};

const generateBarChartSVG = (title: string, data: ChartData): string => {
  const chartWidth = CHART_WIDTH - PADDING * 2;
  const maxValue = Math.max(...data.datasets.flatMap((d) => d.data));
  const groupWidth = chartWidth / data.labels.length;
  const barWidth = (groupWidth / data.datasets.length) * 0.7;
  const barGap = groupWidth * 0.15;

  let svg = createSvgBase(title);
  svg += drawAxes(chartWidth, CHART_INNER_HEIGHT);
  svg += drawGridLines(chartWidth, CHART_INNER_HEIGHT, maxValue);

  data.labels.forEach((label, labelIndex) => {
    const groupX = PADDING + labelIndex * groupWidth + barGap;

    data.datasets.forEach((dataset, datasetIndex) => {
      const value = dataset.data[labelIndex];
      const barHeight = (value / maxValue) * CHART_INNER_HEIGHT;
      const x = groupX + datasetIndex * (barWidth + 5);
      const y = PADDING + CHART_INNER_HEIGHT - barHeight;

      svg += `<rect x="${x}" y="${y}" width="${barWidth}" height="${barHeight}" fill="${colors.primary[datasetIndex % colors.primary.length]}" rx="3"/>`;
      svg += `<text x="${x + barWidth / 2}" y="${y - 10}" font-size="18" text-anchor="middle" fill="${colors.text}">${value}</text>`;
    });

    svg += `<text x="${groupX + (barWidth * data.datasets.length) / 2}" y="${PADDING + CHART_INNER_HEIGHT + 35}" font-size="20" text-anchor="middle" fill="${colors.text}">${label}</text>`;
  });

  svg += drawLegend(data.datasets);
  svg += '</svg>';
  return svg;
};

const generateLineChartSVG = (title: string, data: ChartData): string => {
  const chartWidth = CHART_WIDTH - PADDING * 2;
  const maxValue = Math.max(...data.datasets.flatMap((d) => d.data));
  const xStep = chartWidth / (data.labels.length - 1);

  let svg = createSvgBase(title);
  svg += drawAxes(chartWidth, CHART_INNER_HEIGHT);
  svg += drawGridLines(chartWidth, CHART_INNER_HEIGHT, maxValue, true);

  data.datasets.forEach((dataset, datasetIndex) => {
    const color = colors.primary[datasetIndex % colors.primary.length];

    const pathData = dataset.data
      .map((value, i) => {
        const x = PADDING + i * xStep;
        const y = PADDING + CHART_INNER_HEIGHT - (value / maxValue) * CHART_INNER_HEIGHT;
        return `${i === 0 ? 'M' : 'L'} ${x} ${y}`;
      })
      .join(' ');

    svg += `<path d="${pathData}" fill="none" stroke="${color}" stroke-width="4"/>`;

    dataset.data.forEach((value, i) => {
      const x = PADDING + i * xStep;
      const y = PADDING + CHART_INNER_HEIGHT - (value / maxValue) * CHART_INNER_HEIGHT;

      svg += `<circle cx="${x}" cy="${y}" r="8" fill="${color}"/>`;
      svg += `<text x="${x}" y="${y - 15}" font-size="16" text-anchor="middle" fill="${colors.text}">${value}</text>`;
    });
  });

  data.labels.forEach((label, i) => {
    const x = PADDING + i * xStep;
    svg += `<text x="${x}" y="${PADDING + CHART_INNER_HEIGHT + 35}" font-size="20" text-anchor="middle" fill="${colors.text}">${label}</text>`;
  });

  svg += drawLegend(data.datasets);
  svg += '</svg>';
  return svg;
};

const generatePieChartSVG = (title: string, data: ChartData): string => {
  const centerX = CHART_WIDTH / 2;
  const centerY = CHART_HEIGHT / 2 - 30;
  const radius = Math.min(CHART_WIDTH, CHART_HEIGHT - 200) / 2 - 80;

  const pieData = data.datasets[0]?.data || [];
  const total = pieData.reduce((sum, val) => sum + val, 0);
  let currentAngle = -Math.PI / 2;

  let svg = createSvgBase(title);

  pieData.forEach((value, i) => {
    const percentage = value / total;
    const angle = percentage * 2 * Math.PI;
    const startAngle = currentAngle;
    const endAngle = currentAngle + angle;
    currentAngle = endAngle;

    const x1 = centerX + Math.cos(startAngle) * radius;
    const y1 = centerY + Math.sin(startAngle) * radius;
    const x2 = centerX + Math.cos(endAngle) * radius;
    const y2 = centerY + Math.sin(endAngle) * radius;

    const largeArcFlag = angle > Math.PI ? 1 : 0;
    const path = [`M ${centerX} ${centerY}`, `L ${x1} ${y1}`, `A ${radius} ${radius} 0 ${largeArcFlag} 1 ${x2} ${y2}`, 'Z'].join(' ');

    svg += `<path d="${path}" fill="${colors.primary[i % colors.primary.length]}"/>`;

    const labelAngle = (startAngle + endAngle) / 2;
    const labelRadius = radius * 0.7;
    const labelX = centerX + Math.cos(labelAngle) * labelRadius;
    const labelY = centerY + Math.sin(labelAngle) * labelRadius;

    svg += `<text x="${labelX}" y="${labelY}" font-size="24" font-weight="bold" text-anchor="middle" dominant-baseline="middle" fill="white">${(percentage * 100).toFixed(0)}%</text>`;
  });

  const legendY = CHART_HEIGHT - 60;
  const legendItemWidth = 180;
  const totalItems = data.labels.length;
  const startX = (CHART_WIDTH - totalItems * legendItemWidth) / 2;

  data.labels.forEach((label, i) => {
    const x = startX + i * legendItemWidth;
    svg += `<rect x="${x}" y="${legendY}" width="16" height="16" fill="${colors.primary[i % colors.primary.length]}" rx="2"/>`;
    svg += `<text x="${x + 25}" y="${legendY + 12}" font-size="18" fill="${colors.text}">${label} (${pieData[i]})</text>`;
  });

  svg += '</svg>';
  return svg;
};
