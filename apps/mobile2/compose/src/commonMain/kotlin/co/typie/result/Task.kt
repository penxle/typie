package co.typie.result

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.channelFlow

class Task<P, T, E> internal constructor(
  private val flow: Flow<Event<P, T, E>>,
) {
  internal sealed interface Event<out P, out T, out E> {
    data class Pending<P>(val value: P) : Event<P, Nothing, Nothing>
    data class Settled<T, E>(val result: Result<T, E>) : Event<Nothing, T, E>
  }

  suspend fun collect(
    onPending: suspend (P) -> Unit,
    onSettled: suspend (Result<T, E>) -> Unit,
  ) = flow.collect { event ->
    when (event) {
      is Event.Pending -> onPending(event.value)
      is Event.Settled -> onSettled(event.result)
    }
  }
}

class TaskScope<P, E> @PublishedApi internal constructor(
  private val emitFn: suspend (P) -> Unit,
) {
  suspend fun emit(progress: P) {
    emitFn(progress)
  }

  fun raise(error: E): Nothing {
    throw RaiseException(error)
  }

  @PublishedApi
  internal class RaiseException(val error: Any?) : Exception()
}

fun <P, T, E> task(block: suspend TaskScope<P, E>.() -> T): Task<P, T, E> {
  val flow = channelFlow {
    val scope = TaskScope<P, E> { progress -> send(Task.Event.Pending(progress)) }
    val result: Result<T, E> = try {
      Result.Ok(scope.block())
    } catch (e: TaskScope.RaiseException) {
      @Suppress("UNCHECKED_CAST")
      Result.Err(e.error as E)
    } catch (e: CancellationException) {
      throw e
    } catch (e: Throwable) {
      Result.Exception(e)
    }
    send(Task.Event.Settled(result))
  }
  return Task(flow)
}
