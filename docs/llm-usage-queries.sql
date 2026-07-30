-- LLM 사용량 분석 질의 모음
--
-- 모든 질의는 토큰을 낸다. 금액 = 토큰 × 단가로 직접 계산한다.
-- 어느 토큰이 어느 모델 것인지는 4번 질의의 model 열에서 갈라진다.
--
-- 비용 관련 질의는 두 벌이다:
--   실지출 = 캐시 미스만    (cache_status IS DISTINCT FROM 'HIT')
--   명목   = 전량           (캐시가 없었다면 들었을 양)
-- 둘의 차이가 곧 게이트웨이 캐시 절감량이다.
--
-- 청구 출력 = output_tokens + reasoning_tokens.
-- Vertex AI 경유 Gemini는 completion_tokens에 추론 토큰을 포함하지 않고 따로 보고하며,
-- 셋을 더해야 total_tokens가 된다(2026-07-31 실측: 13 + 1 + 130 = 144).
-- ANALYZE는 effort=medium, META는 effort=low로 추론이 켜져 있어 이 항이 출력 비용의 대부분이다.
-- output_tokens만 세면 출력 비용을 두 자릿수 배수로 과소 계상한다.

-- 1. 유저별 총계
-- run 컬럼(text_length)은 call 조인으로 run당 call 수만큼 복제되므로, run 단위로 먼저 접은 뒤 유저로 합친다.
-- 바로 GROUP BY user_id로 가면 total_chars가 call 수만큼(보통 5~20배) 부풀려진다.
-- cached_input_tokens는 input_tokens의 부분집합이다(할인 단가). 정가 입력 = billed_input - billed_cached_input.
WITH run_totals AS (
  SELECT
    r.user_id,
    r.id,
    r.text_length,
    count(u.id)                                                                     AS calls,
    sum(u.input_tokens)                                                             AS nominal_input,
    sum(coalesce(u.output_tokens, 0) + coalesce(u.reasoning_tokens, 0))             AS nominal_output,
    sum(u.input_tokens) FILTER (WHERE u.cache_status IS DISTINCT FROM 'HIT')        AS billed_input,
    sum(coalesce(u.output_tokens, 0) + coalesce(u.reasoning_tokens, 0))
      FILTER (WHERE u.cache_status IS DISTINCT FROM 'HIT')                          AS billed_output,
    sum(u.cached_input_tokens) FILTER (WHERE u.cache_status IS DISTINCT FROM 'HIT') AS billed_cached_input
  FROM llm_analysis_runs r
  LEFT JOIN llm_call_usage u ON u.run_id = r.id
  GROUP BY r.user_id, r.id, r.text_length
)
SELECT
  user_id,
  count(*)                  AS runs,
  sum(calls)                AS calls,
  sum(text_length)          AS total_chars,
  sum(nominal_input)        AS nominal_input_tokens,
  sum(nominal_output)       AS nominal_output_tokens,
  sum(billed_input)         AS billed_input_tokens,
  sum(billed_output)        AS billed_output_tokens,
  sum(billed_cached_input)  AS billed_cached_input_tokens
FROM run_totals
GROUP BY user_id
ORDER BY billed_input_tokens DESC NULLS LAST;

-- 2. 원고 길이 버킷별 실지출 산포
WITH run_cost AS (
  SELECT
    r.id,
    r.text_length,
    sum(u.input_tokens) FILTER (WHERE u.cache_status IS DISTINCT FROM 'HIT') AS billed_input,
    sum(coalesce(u.output_tokens, 0) + coalesce(u.reasoning_tokens, 0))
      FILTER (WHERE u.cache_status IS DISTINCT FROM 'HIT')                   AS billed_output
  FROM llm_analysis_runs r
  LEFT JOIN llm_call_usage u ON u.run_id = r.id
  GROUP BY r.id, r.text_length
)
SELECT
  (width_bucket(text_length, 0, 100000, 20) - 1) * 5000 AS chars_bucket_start,
  count(*)                                        AS runs,
  round(avg(billed_input))                        AS avg_billed_input,
  percentile_cont(0.5) WITHIN GROUP (ORDER BY billed_input) AS median_billed_input,
  round(avg(billed_output))                       AS avg_billed_output
FROM run_cost
GROUP BY 1
ORDER BY 1;

-- 3-a. 재분석 비율 — 완전 동일 원고 (full_hash)
SELECT repeats, count(*) AS manuscripts
FROM (
  SELECT user_id, full_hash, count(*) AS repeats
  FROM llm_analysis_runs
  GROUP BY user_id, full_hash
) t
GROUP BY repeats
ORDER BY repeats;

-- 3-b. 재분석 비율 — 같은 원고 계열 (prefix_hash)
SELECT repeats, count(*) AS manuscripts
FROM (
  SELECT user_id, prefix_hash, count(*) AS repeats
  FROM llm_analysis_runs
  GROUP BY user_id, prefix_hash
) t
GROUP BY repeats
ORDER BY repeats;

-- 4. 단계별·모델별 비중 (금액 환산의 기준 표)
SELECT
  phase,
  model,
  count(*)                                                                    AS calls,
  sum(input_tokens)                                                           AS nominal_input,
  sum(input_tokens) FILTER (WHERE cache_status IS DISTINCT FROM 'HIT')        AS billed_input,
  sum(coalesce(output_tokens, 0) + coalesce(reasoning_tokens, 0))
    FILTER (WHERE cache_status IS DISTINCT FROM 'HIT')                        AS billed_output,
  sum(output_tokens) FILTER (WHERE cache_status IS DISTINCT FROM 'HIT')       AS billed_completion_only,
  sum(reasoning_tokens) FILTER (WHERE cache_status IS DISTINCT FROM 'HIT')    AS billed_reasoning,
  sum(cached_input_tokens) FILTER (WHERE cache_status IS DISTINCT FROM 'HIT') AS billed_cached_input,
  round(avg(duration_ms))                                                     AS avg_duration_ms
FROM llm_call_usage
GROUP BY phase, model
ORDER BY billed_input DESC NULLS LAST;

-- 5. 중단율과 중단이 태운 비용
-- ABORTED는 유저 취소(패널 닫기·편집 중 자동 취소)와 소켓 단절 양쪽에서 발생한다.
SELECT
  r.state,
  count(DISTINCT r.id)                                                     AS runs,
  sum(u.input_tokens) FILTER (WHERE u.cache_status IS DISTINCT FROM 'HIT') AS billed_input,
  sum(coalesce(u.output_tokens, 0) + coalesce(u.reasoning_tokens, 0))
    FILTER (WHERE u.cache_status IS DISTINCT FROM 'HIT')                   AS billed_output
FROM llm_analysis_runs r
LEFT JOIN llm_call_usage u ON u.run_id = r.id
GROUP BY r.state;

-- 6. 토큰:프롬프트문자 환산 계수
-- input_chars는 원고가 아니라 프롬프트 전체(시스템 프롬프트 + meta 블록 + 인접 요약 + 청크)의 문자 수다.
-- 원고 글자당 단가는 이 질의가 아니라 질의 2(text_length 버킷)에서 나온다.
SELECT
  phase,
  model,
  count(*)                                                      AS calls,
  round(avg(input_tokens::numeric / nullif(input_chars, 0)), 4) AS tokens_per_char
FROM llm_call_usage
WHERE input_tokens IS NOT NULL
GROUP BY phase, model;

-- 7. 캐시 히트율과 절감량
SELECT
  phase,
  cache_status,
  count(*)                                                        AS calls,
  sum(input_tokens)                                               AS input_tokens,
  sum(coalesce(output_tokens, 0) + coalesce(reasoning_tokens, 0)) AS output_tokens
FROM llm_call_usage
GROUP BY phase, cache_status
ORDER BY phase, cache_status;

-- 8. 유저별 실지출 분위수 — quota 라인 후보
-- cached_input_tokens는 input_tokens의 부분집합이다(할인 단가). 정가 입력 = billed_input - billed_cached_input.
WITH per_user AS (
  SELECT
    r.user_id,
    sum(u.input_tokens) FILTER (WHERE u.cache_status IS DISTINCT FROM 'HIT')        AS billed_input,
    sum(coalesce(u.output_tokens, 0) + coalesce(u.reasoning_tokens, 0))
      FILTER (WHERE u.cache_status IS DISTINCT FROM 'HIT')                          AS billed_output,
    sum(u.cached_input_tokens) FILTER (WHERE u.cache_status IS DISTINCT FROM 'HIT') AS billed_cached_input,
    count(DISTINCT r.id)                                                            AS runs
  FROM llm_analysis_runs r
  LEFT JOIN llm_call_usage u ON u.run_id = r.id
  GROUP BY r.user_id
)
SELECT
  percentile_cont(0.5)  WITHIN GROUP (ORDER BY billed_input)        AS p50_input,
  percentile_cont(0.9)  WITHIN GROUP (ORDER BY billed_input)        AS p90_input,
  percentile_cont(0.99) WITHIN GROUP (ORDER BY billed_input)        AS p99_input,
  percentile_cont(0.5)  WITHIN GROUP (ORDER BY billed_cached_input) AS p50_cached_input,
  percentile_cont(0.9)  WITHIN GROUP (ORDER BY billed_cached_input) AS p90_cached_input,
  percentile_cont(0.99) WITHIN GROUP (ORDER BY billed_cached_input) AS p99_cached_input,
  percentile_cont(0.5)  WITHIN GROUP (ORDER BY runs)                AS p50_runs,
  percentile_cont(0.9)  WITHIN GROUP (ORDER BY runs)                AS p90_runs,
  percentile_cont(0.99) WITHIN GROUP (ORDER BY runs)                AS p99_runs
FROM per_user;

-- 9. 계측 커버리지
-- 원장은 V2 파이프라인만 덮는다. 아래 숫자를 같은 기간의 AI Gateway 로그 총 호출 수와
-- 비교하면 계측되지 않은 트래픽(V1 등)의 규모가 나오고, 그게 이 데이터의 신뢰 구간이다.
-- 게이트웨이 로그 조회에는 대시보드에서 발급한 AI Gateway:Read 토큰이 필요하다
-- (wrangler OAuth 토큰으로는 403).
-- created_at은 호출 시각이 아니라 원장 flush 시각이다(단계 경계·청크 단위 일괄 INSERT).
-- 호출 단위 시각이 필요하면 gateway_log_id(ULID, 앞 10자가 밀리초 타임스탬프)로 복원한다.
SELECT
  min(created_at) AS first_flush,
  max(created_at) AS last_flush,
  count(*)        AS ledger_calls
FROM llm_call_usage;

-- 10. documentId 태깅률
SELECT
  count(*) FILTER (WHERE document_id IS NOT NULL) AS tagged,
  count(*)                                        AS total,
  round(100.0 * count(*) FILTER (WHERE document_id IS NOT NULL) / nullif(count(*), 0), 1) AS tagged_pct
FROM llm_analysis_runs;

-- 11. 토큰 산술 검산
-- input + output + reasoning = total 이 provider가 보고한 total_tokens와 어긋나면
-- 우리가 provider의 회계 모델을 잘못 알고 있는 것이다. 0이 아니면 위 질의들을 믿지 말 것.
SELECT
  model,
  count(*) AS calls,
  count(*) FILTER (
    WHERE total_tokens IS NOT NULL
      AND total_tokens <> coalesce(input_tokens, 0) + coalesce(output_tokens, 0) + coalesce(reasoning_tokens, 0)
  ) AS mismatched
FROM llm_call_usage
GROUP BY model;
