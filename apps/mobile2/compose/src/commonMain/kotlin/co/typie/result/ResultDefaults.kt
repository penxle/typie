package co.typie.result

import co.typie.overlay.Toast
import co.typie.overlay.ToastType

const val DEFAULT_ERROR_MESSAGE = "오류가 발생했어요. 잠시 후 다시 시도해주세요."

fun <T, E> Result<T, E>.withDefaultExceptionHandler(toast: Toast): Result<T, E> =
  onException { toast.show(ToastType.Error, DEFAULT_ERROR_MESSAGE) }
