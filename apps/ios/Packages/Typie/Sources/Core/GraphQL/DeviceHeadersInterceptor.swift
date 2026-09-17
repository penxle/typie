import Apollo

struct DeviceHeadersInterceptor: GraphQLInterceptor {
  let headers: @Sendable () -> [String: String]

  func intercept<Request: GraphQLRequest>(
    request: Request, next: NextInterceptorFunction<Request>
  ) async throws -> InterceptorResultStream<Request> {
    var request = request
    request.addHeaders(headers())
    return await next(request)
  }
}
