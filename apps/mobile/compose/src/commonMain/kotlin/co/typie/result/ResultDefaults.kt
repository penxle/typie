package co.typie.result

import co.touchlab.kermit.Logger
import co.typie.domain.subscription.SubscriptionService
import co.typie.graphql.TypieError
import co.typie.network.isRecoverableNetworkError
import co.typie.ui.component.toast.Toast
import co.typie.ui.component.toast.ToastType
import io.sentry.kotlin.multiplatform.Sentry

const val DEFAULT_ERROR_MESSAGE = "오류가 발생했어요. 잠시 후 다시 시도해주세요."

fun <T, E> Result<T, E>.withDefaultExceptionHandler(toast: Toast): Result<T, E> = onException {
  if (it is TypieError && it.code == "subscription_required") {
    SubscriptionService.requestSubscribeSheet()
    return@onException
  }
  Logger.e(it) { "Unhandled exception" }
  if (!it.isRecoverableNetworkError()) {
    Sentry.captureException(it)
  }
  toast.show(ToastType.Error, DEFAULT_ERROR_MESSAGE)
}
