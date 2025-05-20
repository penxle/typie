import 'dart:async';
import 'dart:convert';

import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:gql_exec/gql_exec.dart';
import 'package:gql_link/gql_link.dart';
import 'package:uuid/uuid.dart';
import 'package:web_socket/web_socket.dart';

part 'ws_link.freezed.dart';
part 'ws_link.g.dart';

class WsLink extends Link {
  WsLink({required this.url, this.connectionParams});

  final String url;
  final FutureOr<Map<String, dynamic>> Function()? connectionParams;
  WebSocket? _ws;

  final RequestSerializer serializer = const RequestSerializer();
  final ResponseParser parser = const ResponseParser();
  final Uuid uuid = const Uuid();

  final Map<Request, StreamController<Response>> _subscriptions = {};
  final Map<String, Request> _subscriptionIds = {};

  @override
  Stream<Response> request(Request request, [NextLink? forward]) {
    _connect();

    final streamController = StreamController<Response>.broadcast();

    _subscriptions[request] = streamController;

    return streamController.stream;
  }

  Future<void> _connect() async {
    if (_ws != null) {
      return;
    }

    _ws = await WebSocket.connect(Uri.parse(url), protocols: ['graphql-transport-ws']);
    _ws!.events.listen((event) {
      switch (event) {
        case TextDataReceived(:final text):
          final message = WsMessage.fromJson(jsonDecode(text) as Map<String, dynamic>);
          _handleMessage(message);
        default:
          break;
      }
    });

    final connectionParams = await this.connectionParams?.call();
    _send(WsMessage.connectionInit(payload: connectionParams));
  }

  void _send(WsMessage message) {
    _ws?.sendText(jsonEncode(message));
  }

  void _handleMessage(WsMessage message) {
    switch (message) {
      case _WsConnectionAckMessage():
        for (final request in _subscriptions.keys) {
          final id = uuid.v4();
          _subscriptionIds[id] = request;

          final payload = serializer.serializeRequest(request);

          _send(WsMessage.subscribe(id: id, payload: payload));
        }

      case _WsNextMessage(:final id, :final payload):
        final request = _subscriptionIds[id]!;
        final response = parser.parseResponse(payload);
        _subscriptions[request]!.add(response);

      case _WsErrorMessage(:final id, :final payload):
        final request = _subscriptionIds[id]!;
        final errors = payload.map(parser.parseError);
        _subscriptions[request]!.addError(errors);

      case _WsCompleteMessage(:final id):
        final request = _subscriptionIds[id]!;
        _subscriptions[request]!.close();

      default:
        break;
    }
  }

  @override
  Future<void> dispose() async {
    await _ws?.close();
    await super.dispose();
  }
}

@Freezed(unionKey: 'type', unionValueCase: FreezedUnionCase.snake)
sealed class WsMessage with _$WsMessage {
  const factory WsMessage.connectionInit({Map<String, dynamic>? payload}) = _WsConnectionInitMessage;
  const factory WsMessage.connectionAck() = _WsConnectionAckMessage;
  const factory WsMessage.subscribe({required String id, required Map<String, dynamic> payload}) = _WsSubscribeMessage;
  const factory WsMessage.next({required String id, required Map<String, dynamic> payload}) = _WsNextMessage;
  const factory WsMessage.error({required String id, required List<Map<String, dynamic>> payload}) = _WsErrorMessage;
  const factory WsMessage.complete({required String id}) = _WsCompleteMessage;

  factory WsMessage.fromJson(Map<String, dynamic> json) => _$WsMessageFromJson(json);
}
