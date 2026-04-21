package co.typie.result

sealed interface Result<out T, out E> {
  data class Ok<T>(val value: T) : Result<T, Nothing>

  data class Err<E>(val error: E) : Result<Nothing, E>

  data class Exception(val exception: Throwable) : Result<Nothing, Nothing>
}
