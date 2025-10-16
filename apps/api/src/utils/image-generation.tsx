import path from 'node:path';
import { random } from '@ctrl/tinycolor';
import { renderAsync } from '@resvg/resvg-js';
import dayjs from 'dayjs';
import { and, asc, eq, gte, lt, sql, sum } from 'drizzle-orm';
import ky from 'ky';
import { renderToStaticMarkup } from 'react-dom/server';
import satori from 'satori';
import twemoji from 'twemoji';
import { db, first, Images, PostCharacterCountChanges, Users } from '@/db';

const generateRandomGradient = () => {
  const first = random({
    luminosity: 'bright',
  });

  const second = first.triad()[1];

  return {
    from: first.toHexString(),
    to: second.toHexString(),
  };
};

export const generateRandomAvatar = async () => {
  const gradient = generateRandomGradient();
  const element = (
    <svg viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
      <g>
        <defs>
          <linearGradient id="gradient" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor={gradient.from} />
            <stop offset="100%" stopColor={gradient.to} />
          </linearGradient>
        </defs>
        <rect fill="url(#gradient)" x="0" y="0" width="512" height="512" />
      </g>
    </svg>
  );

  const markup = renderToStaticMarkup(element);
  const rendered = await renderAsync(markup);

  return new File([new Uint8Array(rendered.asPng())], 'avatar.png', { type: 'image/png' });
};

const loadFonts = async <T extends string>(names: T[]) => {
  const load = async (name: string) => {
    const filePath = path.join('/tmp/fonts', `${name}.ttf`);

    try {
      return await Bun.file(filePath).bytes();
    } catch {
      const url = `https://cdn.typie.net/fonts/ttf/${name}.ttf`;
      const resp = await ky.get(url).arrayBuffer();

      await Bun.write(filePath, resp);

      return resp;
    }
  };

  return Object.fromEntries(await Promise.all(names.map(async (name) => [name, await load(name)]))) as Record<T, ArrayBuffer>;
};

const fonts = await loadFonts(['Paperlogy-4Regular', 'Paperlogy-7Bold', 'DeepMindSans-Regular']);

const gray = {
  100: '#F4F4F5',
  400: '#A1A1AA',
  500: '#71717A',
  700: '#3F3F46',
  950: '#09090B',
};

const green = {
  100: '#DCFCE7',
  300: '#7BF1A8',
  500: '#00C951',
  700: '#008236',
  900: '#0D542B',
};

const levelColors = {
  0: gray[100],
  1: green[100],
  2: green[300],
  3: green[500],
  4: green[700],
  5: green[900],
};

type Level = 0 | 1 | 2 | 3 | 4 | 5;

export async function generateActivityImage(userId: string): Promise<Uint8Array> {
  const user = await db
    .select({
      id: Users.id,
      name: Users.name,
      avatarPath: Images.path,
      createdAt: Users.createdAt,
    })
    .from(Users)
    .innerJoin(Images, eq(Images.id, Users.avatarId))
    .where(eq(Users.id, userId))
    .then(first);

  if (!user) {
    throw new Error('User not found');
  }

  const endDate = dayjs.kst().startOf('day');
  const startDate = endDate.subtract(364, 'days');
  const startOfTomorrow = endDate.add(1, 'day');

  const date = sql<string>`DATE(${PostCharacterCountChanges.bucket} AT TIME ZONE 'Asia/Seoul')`.mapWith(dayjs.kst);
  const characterCountChanges = await db
    .select({
      date,
      additions: sum(PostCharacterCountChanges.additions).mapWith(Number),
      deletions: sum(PostCharacterCountChanges.deletions).mapWith(Number),
    })
    .from(PostCharacterCountChanges)
    .where(
      and(
        eq(PostCharacterCountChanges.userId, userId),
        gte(PostCharacterCountChanges.bucket, startDate),
        lt(PostCharacterCountChanges.bucket, startOfTomorrow),
      ),
    )
    .groupBy(date)
    .orderBy(asc(date));

  const activities: { date: dayjs.Dayjs; additions: number; level: Level }[] = [];

  const numbers = characterCountChanges.map(({ additions }) => additions).filter((n) => n > 0);

  let p95 = 0;
  if (numbers.length > 0) {
    const sorted = [...numbers].toSorted((a, b) => a - b);
    const index = Math.floor(sorted.length * 0.95);
    p95 = sorted[Math.min(index, sorted.length - 1)];
  }

  const totalCharacters = characterCountChanges.reduce((sum, change) => sum + change.additions, 0);

  const changes = Object.fromEntries(characterCountChanges.map((change) => [dayjs(change.date).unix(), change]));

  let currentDate = startDate;
  while (!currentDate.isAfter(endDate)) {
    const change = changes[currentDate.unix()];
    if (change) {
      if (change.additions === 0) {
        activities.push({ date: currentDate, additions: 0, level: 0 });
      } else if (p95 === 0) {
        activities.push({ date: currentDate, additions: change.additions, level: 3 });
      } else if (change.additions >= p95) {
        activities.push({ date: currentDate, additions: change.additions, level: 5 });
      } else {
        const ratio = change.additions / p95;
        const level = Math.min(Math.floor(ratio * 4) + 1, 4) as Level;
        activities.push({ date: currentDate, additions: change.additions, level });
      }
    } else {
      activities.push({ date: currentDate, additions: 0, level: 0 });
    }

    currentDate = currentDate.add(1, 'day');
  }

  const activitiesByMonth: Record<string, typeof activities> = {};
  const monthNames = ['1월', '2월', '3월', '4월', '5월', '6월', '7월', '8월', '9월', '10월', '11월', '12월'];

  for (let i = 0; i < 12; i++) {
    const monthKey = endDate.subtract(i, 'month').format('YYYY-MM');
    activitiesByMonth[monthKey] = [];
  }

  activities.forEach((activity) => {
    const monthKey = activity.date.format('YYYY-MM');
    if (activitiesByMonth[monthKey]) {
      activitiesByMonth[monthKey].push(activity);
    }
  });

  const sortedMonths = Object.keys(activitiesByMonth)
    .toSorted()
    .map((monthKey) => ({
      key: monthKey,
      name: monthNames[Number.parseInt(monthKey.split('-')[1]) - 1],
      year: monthKey.split('-')[0],
      activities: activitiesByMonth[monthKey],
    }));

  const avatarBuffer = await ky(`https://typie.net/images/${user.avatarPath}?s=256&f=png`).bytes();
  const avatarBase64 = avatarBuffer.toBase64();

  const node = (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        width: '1000px',
        height: '1000px',
        backgroundColor: '#FFFFFF',
        fontFamily: 'Paperlogy',
        padding: '40px',
        justifyContent: 'space-between',
      }}
    >
      <div
        style={{
          display: 'flex',
          flexDirection: 'row',
          alignItems: 'center',
          justifyContent: 'space-between',
          marginBottom: '20px',
        }}
      >
        <div
          style={{
            display: 'flex',
            flexDirection: 'row',
            alignItems: 'center',
            gap: '20px',
          }}
        >
          <div
            style={{
              display: 'flex',
              width: '80px',
              height: '80px',
              borderRadius: '40px',
              overflow: 'hidden',
              backgroundColor: gray[100],
            }}
          >
            <img src={`data:image/png;base64,${avatarBase64}`} width={80} height={80} style={{ objectFit: 'cover' }} />
          </div>

          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: '4px',
            }}
          >
            <div
              style={{
                display: 'flex',
                fontSize: '36px',
                fontWeight: 700,
                color: gray[950],
              }}
            >
              {user.name}
            </div>
            <div
              style={{
                display: 'flex',
                fontSize: '24px',
                fontWeight: 400,
                color: gray[500],
              }}
            >
              나의 글쓰기 발자취
            </div>
          </div>
        </div>
      </div>

      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '40px',
        }}
      >
        <div
          style={{
            display: 'flex',
            flexDirection: 'row',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <div
            style={{
              display: 'flex',
              fontSize: '24px',
              fontWeight: 400,
              color: gray[500],
            }}
          >
            누적 {totalCharacters.toLocaleString()}자
          </div>

          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
              fontSize: '20px',
              fontWeight: 400,
              color: gray[500],
            }}
          >
            <span>적음</span>
            <div style={{ display: 'flex', gap: '6px' }}>
              {[0, 1, 2, 3, 4, 5].map((level) => (
                <div
                  key={level}
                  style={{
                    width: '24px',
                    height: '24px',
                    backgroundColor: levelColors[level as Level],
                    borderRadius: '4px',
                  }}
                />
              ))}
            </div>
            <span>많음</span>
          </div>
        </div>

        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '14px',
            alignItems: 'center',
          }}
        >
          {[0, 1, 2].map((rowIndex) => (
            <div
              key={rowIndex}
              style={{
                display: 'flex',
                gap: '14px',
              }}
            >
              {sortedMonths.slice(rowIndex * 4, (rowIndex + 1) * 4).map((month) => {
                const firstDay = dayjs(month.key + '-01');
                const firstDayOfWeek = firstDay.day();
                const daysInMonth = firstDay.daysInMonth();

                const emptyCells = Array.from({ length: firstDayOfWeek }, () => null);

                const activityMap = Object.fromEntries(month.activities.map((a) => [a.date.date(), a]));

                return (
                  <div
                    key={month.key}
                    style={{
                      display: 'flex',
                      flexDirection: 'column',
                      gap: '8px',
                      width: '217px',
                    }}
                  >
                    <div
                      style={{
                        fontSize: '20px',
                        fontWeight: 700,
                        color: gray[700],
                        marginBottom: '4px',
                      }}
                    >
                      {month.name}
                    </div>

                    <div
                      style={{
                        display: 'flex',
                        flexWrap: 'wrap',
                        gap: '3px',
                        width: '217px',
                      }}
                    >
                      {emptyCells.map((_, i) => (
                        <div
                          key={`empty-${i}`}
                          style={{
                            width: '28px',
                            height: '28px',
                          }}
                        />
                      ))}

                      {Array.from({ length: daysInMonth }, (_, i) => i + 1).map((day) => {
                        const activity = activityMap[day] || { level: 0 };
                        return (
                          <div
                            key={day}
                            style={{
                              width: '28px',
                              height: '28px',
                              backgroundColor: levelColors[activity.level] ?? levelColors[0],
                              borderRadius: '4px',
                            }}
                          />
                        );
                      })}
                    </div>
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      </div>

      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginTop: '20px',
          fontSize: '28px',
          fontWeight: 400,
          color: gray[400],
        }}
      >
        <span>TYPIE &mdash; 작가를 위한 글쓰기 도구</span>
        <span style={{ fontFamily: 'DeepMindSans' }}>https://typie.co</span>
      </div>
    </div>
  );

  const scale = 2;
  const svg = await satori(node, {
    width: 1000,
    height: 1000,
    fonts: [
      { name: 'Paperlogy', data: fonts['Paperlogy-4Regular'], weight: 400 },
      { name: 'Paperlogy', data: fonts['Paperlogy-7Bold'], weight: 700 },
      { name: 'DeepMindSans', data: fonts['DeepMindSans-Regular'], weight: 400 },
    ],
    loadAdditionalAsset: async (code, segment) => {
      const svg = await (async () => {
        if (code === 'emoji') {
          const codepoint = twemoji.convert.toCodePoint(segment);
          try {
            return await ky(`https://cdn.jsdelivr.net/gh/twitter/twemoji@14.0.2/assets/svg/${codepoint}.svg`).text();
          } catch {
            try {
              return await ky(`https://cdn.jsdelivr.net/gh/twitter/twemoji@14.0.2/assets/svg/${codepoint.split('-')[0]}.svg`).text();
            } catch {
              return '<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" />';
            }
          }
        }

        return '<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" />';
      })();

      return 'data:image/svg+xml,' + encodeURIComponent(svg);
    },
  });

  const img = await renderAsync(svg, {
    font: { loadSystemFonts: false },
    imageRendering: 0,
    shapeRendering: 2,
    textRendering: 1,
    fitTo: {
      mode: 'width',
      value: 1000 * scale,
    },
  });

  return Uint8Array.from(img.asPng());
}
