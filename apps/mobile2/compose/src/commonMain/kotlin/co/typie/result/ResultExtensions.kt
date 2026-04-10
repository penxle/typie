package co.typie.result

inline fun <T, E> Result<T, E>.onOk(block: (T) -> Unit): Result<T, E> =
  also { if (this is Result.Ok) block(value) }

inline fun <T, E> Result<T, E>.onErr(block: (E) -> Unit): Result<T, E> =
  also { if (this is Result.Err) block(error) }

inline fun <T, E> Result<T, E>.onException(block: (Throwable) -> Unit): Result<T, E> =
  also { if (this is Result.Exception) block(exception) }

val <T, E> Result<T, E>.isOk: Boolean get() = this is Result.Ok

inline fun <T, E, R> Result<T, E>.fold(
  onOk: (T) -> R,
  onErr: (E) -> R,
  onException: (Throwable) -> R,
): R = when (this) {
  is Result.Ok -> onOk(value)
  is Result.Err -> onErr(error)
  is Result.Exception -> onException(exception)
}
