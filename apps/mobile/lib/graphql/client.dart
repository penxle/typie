import 'dart:async';
import 'dart:isolate';

import 'package:dio/dio.dart';
import 'package:dio_http2_adapter/dio_http2_adapter.dart';
import 'package:ferry/ferry.dart';
import 'package:ferry/ferry_isolate.dart';
import 'package:flutter/foundation.dart';
import 'package:gql_dio_link/gql_dio_link.dart';
import 'package:gql_error_link/gql_error_link.dart';
import 'package:gql_exec/gql_exec.dart' hide GraphQLError;
import 'package:injectable/injectable.dart';
import 'package:sentry/sentry_io.dart';
import 'package:typie/env.dart';
import 'package:typie/graphql/__generated__/create_ws_session_mutation.req.gql.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart' show possibleTypesMap;
import 'package:typie/graphql/auth_link.dart';
import 'package:typie/graphql/cookie_link.dart';
import 'package:typie/graphql/error.dart';
import 'package:typie/graphql/message.dart';
import 'package:typie/graphql/ws_link.dart';
import 'package:typie/services/auth.dart';

@singleton
class GraphQLClient {
  GraphQLClient._(this._client);

  final IsolateClient _client;

  IsolateClient get raw => _client;

  @FactoryMethod(preResolve: true)
  static Future<GraphQLClient> create(Auth auth) async {
    SendPort? sendPort;

    auth.addListener(() {
      final accessToken = switch (auth.value) {
        Authenticated(:final accessToken) => accessToken,
        _ => null,
      };

      sendPort?.send(GraphQLAccessTokenMessage(accessToken));
    });

    final accessToken = switch (auth.value) {
      Authenticated(:final accessToken) => accessToken,
      _ => null,
    };

    final client = await IsolateClient.create(
      _createClient,
      params: _CreateClientParams(accessToken: accessToken),
      messageHandler: (message) async {
        switch (message) {
          case GraphQLPortMessage(:final port):
            sendPort = port;
          case GraphQLCookieMessage(:final cookie):
            if (cookie.name == 'typie-st') {
              await auth.login(cookie.value);
            }
        }
      },
    );

    return GraphQLClient._(client);
  }

  Future<TData> request<TData, TVars>(OperationRequest<TData, TVars> request) async {
    OperationResponse<TData, TVars> resp;

    try {
      resp = await _client.request(request).first;
    } catch (err) {
      throw OperationError.exception(err);
    }

    if (resp.linkException != null) {
      throw OperationError.exception(resp.linkException!);
    }

    if (resp.graphqlErrors?.isNotEmpty ?? false) {
      final error = resp.graphqlErrors![0];
      throw OperationError.graphql(GraphQLError(error));
    }

    return resp.data as TData;
  }

  Stream<TData> subscribe<TData, TVars>(OperationRequest<TData, TVars> request) {
    return _client
        .request(request)
        .where((response) => response.data != null)
        .map((response) => response.data as TData);
  }

  Future<void> refetch<TData, TVars>(OperationRequest<TData, TVars> request) {
    return _client.addRequestToRequestController(request);
  }

  Future<void> dispose() async {
    await _client.dispose();
  }
}

class _CreateClientParams {
  _CreateClientParams({this.accessToken});

  final String? accessToken;
}

Future<Client> _createClient(_CreateClientParams params, SendPort? sendPort) async {
  Isolate.current.addSentryErrorListener();

  final receivePort = ReceivePort();
  sendPort?.send(GraphQLMessage.port(receivePort.sendPort));

  var accessToken = params.accessToken;

  receivePort.listen((message) {
    switch (message) {
      case GraphQLAccessTokenMessage(:final token):
        accessToken = token;
    }
  });

  final dio = kDebugMode
      ? (Dio()..httpClientAdapter = HttpClientAdapter())
      : (Dio()..httpClientAdapter = Http2Adapter(ConnectionManager()));

  final link = Link.from([
    ErrorLink(
      onGraphQLError: (request, forward, response) {
        unawaited(Sentry.captureException(response.errors!.first));
        return null;
      },
      onException: (request, forward, exception) {
        unawaited(Sentry.captureException(exception));
        return null;
      },
    ),
    authLink(getAccessToken: () => accessToken),
    cookieLink(
      setter: (cookie) {
        sendPort?.send(GraphQLMessage.cookie(cookie));
      },
    ),
    Link.split(
      (request) => request.operation.getOperationType() == OperationType.subscription,
      WsLink(
        url: '${Env.wsUrl}/graphql',
        connectionParams: () async {
          final client = Client(
            link: Link.from([
              authLink(getAccessToken: () => accessToken),
              DioLink('${Env.apiUrl}/graphql', client: dio),
            ]),
          );

          final result = await client.request(GCreateWsSession_MutationReq()).first;
          return {'session': result.data!.createWsSession};
        },
      ),
      DioLink('${Env.apiUrl}/graphql', client: dio),
    ),
  ]);

  final cache = Cache(possibleTypes: possibleTypesMap);

  return Client(
    link: link,
    cache: cache,
    defaultFetchPolicies: {
      OperationType.query: FetchPolicy.CacheAndNetwork,
      OperationType.mutation: FetchPolicy.NetworkOnly,
      OperationType.subscription: FetchPolicy.NetworkOnly,
    },
  );
}
