import Apollo

struct BearerInterceptor: GraphQLInterceptor {
  let accessToken: @Sendable () -> String?

  func intercept<Request: GraphQLRequest>(
    request: Request, next: NextInterceptorFunction<Request>
  ) async throws -> InterceptorResultStream<Request> {
    guard let token = accessToken() else { return await next(request) }
    var request = request
    request.addHeader(name: "Authorization", value: "Bearer \(token)")
    return await next(request)
  }
}
