<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { QueryString } from '@typie/ui/state';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { untrack } from 'svelte';

  type Granularity = 'day' | 'week' | 'month';

  const startDate = new QueryString('start', dayjs.kst().subtract(30, 'day').format('YYYY-MM-DD'));
  const endDate = new QueryString('end', dayjs.kst().format('YYYY-MM-DD'));
  const granularity = new QueryString<Granularity>('granularity', 'day');

  type ColumnData = Record<string, string | number | null>;

  type ColumnDefinition = {
    key: string;
    label: string;
    width: string;
    query: (start: string, end: string, gran: Granularity) => { query: string; params: unknown[] };
    format?: (value: unknown) => string;
  };

  const columns: ColumnDefinition[] = [
    {
      key: 'total_active_users',
      label: '총 활성 가입자',
      width: '11%',
      query: (start, end, gran) => {
        const truncFunc = gran === 'day' ? 'day' : gran === 'week' ? 'week' : 'month';
        return {
          query: `
            WITH periods AS (
              SELECT date_trunc($1, d)::date AS period
              FROM generate_series($2::date, $3::date, $4) AS d
            )
            SELECT
              periods.period,
              COUNT(users.id) AS value
            FROM periods
            LEFT JOIN users ON (users.created_at AT TIME ZONE 'Asia/Seoul')::date <= periods.period + ($4)::interval - interval '1 second'
              AND users.state = 'ACTIVE'
            GROUP BY periods.period
            ORDER BY periods.period
          `,
          params: [truncFunc, start, end, `1 ${gran}`],
        };
      },
      format: (value) => comma(Number(value)),
    },
    {
      key: 'new_users',
      label: '신규 가입자',
      width: '11%',
      query: (start, end, gran) => {
        const truncFunc = gran === 'day' ? 'day' : gran === 'week' ? 'week' : 'month';
        return {
          query: `
            WITH periods AS (
              SELECT date_trunc($1, d)::date AS period
              FROM generate_series($2::date, $3::date, $4) AS d
            )
            SELECT
              periods.period,
              COUNT(users.id) AS value
            FROM periods
            LEFT JOIN users ON date_trunc($1, users.created_at AT TIME ZONE 'Asia/Seoul')::date = periods.period
            GROUP BY periods.period
            ORDER BY periods.period
          `,
          params: [truncFunc, start, end, `1 ${gran}`],
        };
      },
      format: (value) => comma(Number(value)),
    },
    {
      key: 'total_active_subscriptions',
      label: '총 활성 구독자',
      width: '11%',
      query: (start, end, gran) => {
        const truncFunc = gran === 'day' ? 'day' : gran === 'week' ? 'week' : 'month';
        return {
          query: `
            WITH periods AS (
              SELECT date_trunc($1, d)::date AS period
              FROM generate_series($2::date, $3::date, $4) AS d
            )
            SELECT
              periods.period,
              COUNT(subscriptions.id) AS value
            FROM periods
            LEFT JOIN subscriptions ON (subscriptions.starts_at AT TIME ZONE 'Asia/Seoul')::date <= periods.period + ($4)::interval - interval '1 second'
              AND (subscriptions.expires_at AT TIME ZONE 'Asia/Seoul')::date > periods.period
              AND subscriptions.state IN ('ACTIVE', 'WILL_EXPIRE', 'IN_GRACE_PERIOD')
            GROUP BY periods.period
            ORDER BY periods.period
          `,
          params: [truncFunc, start, end, `1 ${gran}`],
        };
      },
      format: (value) => comma(Number(value)),
    },
    {
      key: 'new_subscriptions',
      label: '신규 구독자',
      width: '11%',
      query: (start, end, gran) => {
        const truncFunc = gran === 'day' ? 'day' : gran === 'week' ? 'week' : 'month';
        return {
          query: `
            WITH periods AS (
              SELECT date_trunc($1, d)::date AS period
              FROM generate_series($2::date, $3::date, $4) AS d
            )
            SELECT
              periods.period,
              COUNT(subscriptions.id) AS value
            FROM periods
            LEFT JOIN subscriptions ON date_trunc($1, subscriptions.created_at AT TIME ZONE 'Asia/Seoul')::date = periods.period
            GROUP BY periods.period
            ORDER BY periods.period
          `,
          params: [truncFunc, start, end, `1 ${gran}`],
        };
      },
      format: (value) => comma(Number(value)),
    },
    {
      key: 'expired_subscriptions',
      label: '구독 만료자',
      width: '11%',
      query: (start, end, gran) => {
        const truncFunc = gran === 'day' ? 'day' : gran === 'week' ? 'week' : 'month';
        return {
          query: `
            WITH periods AS (
              SELECT date_trunc($1, d)::date AS period
              FROM generate_series($2::date, $3::date, $4) AS d
            )
            SELECT
              periods.period,
              COUNT(subscriptions.id) AS value
            FROM periods
            LEFT JOIN subscriptions ON date_trunc($1, subscriptions.expires_at AT TIME ZONE 'Asia/Seoul')::date = periods.period
            GROUP BY periods.period
            ORDER BY periods.period
          `,
          params: [truncFunc, start, end, `1 ${gran}`],
        };
      },
      format: (value) => comma(Number(value)),
    },
    {
      key: 'active_users',
      label: '활동 사용자',
      width: '11%',
      query: (start, end, gran) => {
        const truncFunc = gran === 'day' ? 'day' : gran === 'week' ? 'week' : 'month';
        return {
          query: `
            WITH periods AS (
              SELECT date_trunc($1, d)::date AS period
              FROM generate_series($2::date, $3::date, $4) AS d
            )
            SELECT
              periods.period,
              COUNT(DISTINCT post_character_count_changes.user_id) AS value
            FROM periods
            LEFT JOIN post_character_count_changes ON date_trunc($1, post_character_count_changes.bucket AT TIME ZONE 'Asia/Seoul')::date = periods.period
            GROUP BY periods.period
            ORDER BY periods.period
          `,
          params: [truncFunc, start, end, `1 ${gran}`],
        };
      },
      format: (value) => comma(Number(value)),
    },
    {
      key: 'payment_count',
      label: '결제 건수',
      width: '11%',
      query: (start, end, gran) => {
        const truncFunc = gran === 'day' ? 'day' : gran === 'week' ? 'week' : 'month';
        return {
          query: `
            WITH periods AS (
              SELECT date_trunc($1, d)::date AS period
              FROM generate_series($2::date, $3::date, $4) AS d
            )
            SELECT
              periods.period,
              COUNT(payment_records.id) AS value
            FROM periods
            LEFT JOIN payment_records ON date_trunc($1, payment_records.created_at AT TIME ZONE 'Asia/Seoul')::date = periods.period
              AND payment_records.outcome = 'SUCCESS'
            GROUP BY periods.period
            ORDER BY periods.period
          `,
          params: [truncFunc, start, end, `1 ${gran}`],
        };
      },
      format: (value) => comma(Number(value)),
    },
    {
      key: 'payment_amount',
      label: '결제 금액',
      width: '11%',
      query: (start, end, gran) => {
        const truncFunc = gran === 'day' ? 'day' : gran === 'week' ? 'week' : 'month';
        return {
          query: `
            WITH periods AS (
              SELECT date_trunc($1, d)::date AS period
              FROM generate_series($2::date, $3::date, $4) AS d
            )
            SELECT
              periods.period,
              COALESCE(SUM(payment_records.billing_amount), 0) AS value
            FROM periods
            LEFT JOIN payment_records ON date_trunc($1, payment_records.created_at AT TIME ZONE 'Asia/Seoul')::date = periods.period
              AND payment_records.outcome = 'SUCCESS'
            GROUP BY periods.period
            ORDER BY periods.period
          `,
          params: [truncFunc, start, end, `1 ${gran}`],
        };
      },
      format: (value) => comma(Number(value)) + '원',
    },
  ];

  let columnData = $state<Record<string, ColumnData>>({});
  let loadingStates = $state<Record<string, boolean>>({});

  async function fetchColumnData(column: ColumnDefinition) {
    const key = column.key;
    loadingStates[key] = true;

    try {
      const { query, params } = untrack(() => column.query(startDate.current, endDate.current, granularity.current));
      const response = await fetch('/graphql', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          query: `
            query AdminRawQuery($query: String!, $params: [JSON!]) {
              adminRawQuery(query: $query, params: $params)
            }
          `,
          variables: { query, params },
        }),
      });

      if (!response.ok) {
        throw new Error('Failed to fetch data');
      }

      const result = await response.json();
      const data: ColumnData = {};

      for (const row of result.data.adminRawQuery) {
        data[row.period] = row.value ?? null;
      }

      columnData[key] = data;
    } catch (err) {
      console.error(`Error fetching ${key}:`, err);
      columnData[key] = {};
    } finally {
      loadingStates[key] = false;
    }
  }

  function loadAllData() {
    columns.forEach((column) => {
      fetchColumnData(column);
    });
  }

  $effect(() => {
    void startDate.current;
    void endDate.current;
    void granularity.current;

    loadAllData();
  });

  const periods = $derived(() => {
    const start = dayjs.kst(startDate.current);
    const end = dayjs.kst(endDate.current);
    const gran = granularity.current;
    const result: string[] = [];

    let current = start;
    while (current.isBefore(end) || current.isSame(end, 'day')) {
      if (gran === 'day') {
        result.push(current.format('YYYY-MM-DD'));
        current = current.add(1, 'day');
      } else if (gran === 'week') {
        result.push(current.startOf('week').format('YYYY-MM-DD'));
        current = current.add(1, 'week');
      } else {
        result.push(current.startOf('month').format('YYYY-MM-DD'));
        current = current.add(1, 'month');
      }
    }

    return result;
  });

  const formatPeriod = (period: string) => {
    const date = dayjs.kst(period);
    if (granularity.current === 'day') {
      return date.format('YYYY-MM-DD');
    } else if (granularity.current === 'week') {
      return date.format('YYYY-MM-DD') + ' (주)';
    } else {
      return date.format('YYYY-MM');
    }
  };

  type DatePreset = {
    label: string;
    start: () => string;
    end: () => string;
  };

  const presets: DatePreset[] = [
    {
      label: '오늘',
      start: () => dayjs.kst().format('YYYY-MM-DD'),
      end: () => dayjs.kst().format('YYYY-MM-DD'),
    },
    {
      label: '어제',
      start: () => dayjs.kst().subtract(1, 'day').format('YYYY-MM-DD'),
      end: () => dayjs.kst().subtract(1, 'day').format('YYYY-MM-DD'),
    },
    {
      label: '최근 7일',
      start: () => dayjs.kst().subtract(7, 'day').format('YYYY-MM-DD'),
      end: () => dayjs.kst().format('YYYY-MM-DD'),
    },
    {
      label: '최근 30일',
      start: () => dayjs.kst().subtract(30, 'day').format('YYYY-MM-DD'),
      end: () => dayjs.kst().format('YYYY-MM-DD'),
    },
    {
      label: '최근 3개월',
      start: () => dayjs.kst().subtract(3, 'month').format('YYYY-MM-DD'),
      end: () => dayjs.kst().format('YYYY-MM-DD'),
    },
    {
      label: '최근 6개월',
      start: () => dayjs.kst().subtract(6, 'month').format('YYYY-MM-DD'),
      end: () => dayjs.kst().format('YYYY-MM-DD'),
    },
    {
      label: '최근 1년',
      start: () => dayjs.kst().subtract(1, 'year').format('YYYY-MM-DD'),
      end: () => dayjs.kst().format('YYYY-MM-DD'),
    },
  ];

  function applyPreset(preset: DatePreset) {
    startDate.current = preset.start();
    endDate.current = preset.end();
  }
</script>

<div class={flex({ flexDirection: 'column', gap: '24px', color: 'amber.500' })}>
  <div>
    <h2 class={css({ fontSize: '18px', color: 'amber.500' })}>STATISTICS DASHBOARD</h2>
    <p class={css({ marginTop: '8px', fontSize: '13px', color: 'amber.400' })}>SYSTEM METRICS AND ANALYTICS</p>
  </div>

  <div
    class={css({
      borderWidth: '2px',
      borderColor: 'amber.500',
      backgroundColor: 'gray.900',
    })}
  >
    <div class={css({ padding: '20px', borderBottomWidth: '2px', borderColor: 'amber.500' })}>
      <div class={flex({ flexDirection: 'column', gap: '16px' })}>
        <div class={flex({ gap: '8px', flexWrap: 'wrap' })}>
          {#each presets as preset (preset.label)}
            <button
              class={css({
                paddingX: '12px',
                paddingY: '6px',
                borderWidth: '1px',
                borderColor: 'amber.500',
                backgroundColor: 'transparent',
                color: 'amber.500',
                fontSize: '11px',
                cursor: 'pointer',
                _hover: {
                  backgroundColor: 'amber.500',
                  color: 'gray.900',
                },
              })}
              onclick={() => applyPreset(preset)}
              type="button"
            >
              {preset.label.toUpperCase()}
            </button>
          {/each}
        </div>

        <div class={flex({ gap: '16px', alignItems: 'flex-end', flexWrap: 'wrap' })}>
          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            <label class={css({ fontSize: '11px', color: 'amber.400' })} for="start-date">START DATE</label>
            <input
              id="start-date"
              class={css({
                paddingX: '12px',
                paddingY: '8px',
                borderWidth: '2px',
                borderColor: 'amber.500',
                backgroundColor: 'gray.800',
                color: 'amber.500',
                fontSize: '13px',
                outline: 'none',
                caretColor: 'amber.500',
                _focus: {
                  borderColor: 'amber.400',
                },
              })}
              type="date"
              bind:value={startDate.current}
            />
          </div>

          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            <label class={css({ fontSize: '11px', color: 'amber.400' })} for="end-date">END DATE</label>
            <input
              id="end-date"
              class={css({
                paddingX: '12px',
                paddingY: '8px',
                borderWidth: '2px',
                borderColor: 'amber.500',
                backgroundColor: 'gray.800',
                color: 'amber.500',
                fontSize: '13px',
                outline: 'none',
                caretColor: 'amber.500',
                _focus: {
                  borderColor: 'amber.400',
                },
              })}
              type="date"
              bind:value={endDate.current}
            />
          </div>

          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            <label class={css({ fontSize: '11px', color: 'amber.400' })} for="granularity">GRANULARITY</label>
            <select
              id="granularity"
              class={css({
                paddingX: '12px',
                paddingY: '8px',
                borderWidth: '2px',
                borderColor: 'amber.500',
                backgroundColor: 'gray.800',
                color: 'amber.500',
                fontSize: '13px',
                outline: 'none',
                cursor: 'pointer',
                _focus: {
                  borderColor: 'amber.400',
                },
              })}
              bind:value={granularity.current}
            >
              <option value="day">DAILY</option>
              <option value="week">WEEKLY</option>
              <option value="month">MONTHLY</option>
            </select>
          </div>
        </div>
      </div>
    </div>

    <div class={css({ overflowX: 'auto' })}>
      <table class={css({ width: 'full', borderCollapse: 'collapse', tableLayout: 'fixed', minWidth: '1200px' })}>
        <thead>
          <tr class={css({ borderBottomWidth: '2px', borderColor: 'amber.500' })}>
            <th
              style="width: 12%"
              class={css({
                paddingX: '20px',
                paddingY: '16px',
                fontSize: '11px',
                fontFamily: 'mono',
                fontWeight: 'normal',
                color: 'amber.500',
                textAlign: 'left',
              })}
            >
              시간
            </th>
            {#each columns as column (column.key)}
              <th
                style={`width: ${column.width}`}
                class={css({
                  paddingX: '20px',
                  paddingY: '16px',
                  fontSize: '11px',
                  fontFamily: 'mono',
                  fontWeight: 'normal',
                  color: 'amber.500',
                  textAlign: 'left',
                })}
              >
                {column.label}
              </th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#if periods().length > 0}
            {#each periods() as period, i (period)}
              <tr
                class={css({
                  borderBottomWidth: i < periods().length - 1 ? '1px' : '0',
                  borderColor: 'gray.800',
                  _hover: {
                    backgroundColor: 'gray.800',
                  },
                })}
              >
                <td class={css({ padding: '20px', fontSize: '12px', color: 'amber.500' })}>
                  {formatPeriod(period)}
                </td>
                {#each columns as column (column.key)}
                  <td class={css({ padding: '20px', fontSize: '12px', color: 'amber.500' })}>
                    {#if loadingStates[column.key]}
                      <span class={css({ color: 'amber.400' })}>LOADING...</span>
                    {:else if column.format}
                      {column.format(columnData[column.key]?.[period] ?? 0)}
                    {:else}
                      {columnData[column.key]?.[period] ?? 0}
                    {/if}
                  </td>
                {/each}
              </tr>
            {/each}
          {:else}
            <tr>
              <td class={css({ padding: '64px', textAlign: 'center' })} colspan={columns.length + 1}>
                <div class={css({ fontSize: '13px', fontFamily: 'mono', color: 'amber.400' })}>NO DATA</div>
              </td>
            </tr>
          {/if}
        </tbody>
      </table>
    </div>
  </div>
</div>
