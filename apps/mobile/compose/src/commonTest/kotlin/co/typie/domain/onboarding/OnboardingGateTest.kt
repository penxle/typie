package co.typie.domain.onboarding

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.time.Duration.Companion.hours
import kotlin.time.Instant
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

class OnboardingGateTest {
  private val now = Instant.fromEpochMilliseconds(1_700_000_000_000)
  private val emptyPrefs = JsonObject(emptyMap())
  private val completedPrefs =
    JsonObject(mapOf("mobileOnboardingCompletedAt" to JsonPrimitive("2026-07-01T00:00:00Z")))

  @Test
  fun `신규 가입 + 미완료면 표시`() {
    assertTrue(shouldShowOnboarding(createdAt = now - 1.hours, preferences = emptyPrefs, now = now))
  }

  @Test
  fun `가입 24시간 경과면 미표시`() {
    assertFalse(
      shouldShowOnboarding(createdAt = now - 25.hours, preferences = emptyPrefs, now = now)
    )
  }

  @Test
  fun `가입 24시간 직전 경계는 표시`() {
    assertTrue(
      shouldShowOnboarding(
        createdAt = now - 24.hours + 1.hours / 60,
        preferences = emptyPrefs,
        now = now,
      )
    )
  }

  @Test
  fun `완료 기록 있으면 신규여도 미표시`() {
    assertFalse(
      shouldShowOnboarding(createdAt = now - 1.hours, preferences = completedPrefs, now = now)
    )
  }

  @Test
  fun `완료 키가 다른 타입이어도 존재하면 미표시`() {
    val weird = JsonObject(mapOf("mobileOnboardingCompletedAt" to JsonPrimitive(123)))
    assertFalse(shouldShowOnboarding(createdAt = now - 1.hours, preferences = weird, now = now))
  }
}
