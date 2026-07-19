package co.typie.domain.onboarding

import kotlin.time.Duration.Companion.hours
import kotlin.time.Instant
import kotlinx.serialization.json.JsonObject

internal const val ONBOARDING_COMPLETED_KEY = "mobileOnboardingCompletedAt"
private val NEW_USER_WINDOW = 24.hours

fun shouldShowOnboarding(createdAt: Instant, preferences: JsonObject, now: Instant): Boolean =
  now - createdAt < NEW_USER_WINDOW && ONBOARDING_COMPLETED_KEY !in preferences
