package co.typie.result

import kotlinx.coroutines.CancellationException

class ResultScope<E> {
  fun raise(error: E): Nothing {
    throw RaiseException(error)
  }

  fun <T> Result<T, E>.unwrap(): T = when (this) {
    is Result.Ok -> value
    is Result.Err -> raise(error)
    is Result.Exception -> throw exception
  }

  @PublishedApi
  internal class RaiseException(val error: Any?) : Exception()
}

inline fun <T, E> result(block: ResultScope<E>.() -> T): Result<T, E> =
  try {
    Result.Ok(ResultScope<E>().block())
  } catch (e: ResultScope.RaiseException) {
    @Suppress("UNCHECKED_CAST")
    Result.Err(e.error as E)
  } catch (e: CancellationException) {
    throw e
  } catch (e: Throwable) {
    Result.Exception(e)
  }

inline fun <T, E> loading(
  noinline set: (Boolean) -> Unit,
  block: ResultScope<E>.() -> T,
): Result<T, E> {
  set(true)
  return result<T, E>(block).also { set(false) }
}
