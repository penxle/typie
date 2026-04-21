package co.typie.contract

interface Loadable<T> {
  val state: LoadableState<T>

  fun refetch()
}

interface LoadableState<out T> {
  interface Idle : LoadableState<Nothing> {
    companion object : Idle
  }

  interface Loading : LoadableState<Nothing> {
    companion object : Loading
  }

  interface Success<out T> : LoadableState<T> {
    val data: T

    companion object {
      operator fun <T> invoke(data: T): Success<T> = SuccessImpl(data)
    }
  }

  interface Error : LoadableState<Nothing> {
    val exception: Throwable

    companion object {
      operator fun invoke(exception: Throwable): Error = ErrorImpl(exception)
    }
  }
}

private data class SuccessImpl<T>(override val data: T) : LoadableState.Success<T>

private data class ErrorImpl(override val exception: Throwable) : LoadableState.Error
