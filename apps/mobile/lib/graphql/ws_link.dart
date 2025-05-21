import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:gql_exec/gql_exec.dart';
import 'package:gql_link/gql_link.dart';
import 'package:uuid/uuid.dart';
import 'package:web_socket/web_socket.dart';

part 'ws_link.freezed.dart';
part 'ws_link.g.dart';

class WsLink extends Link {
  WsLink({required this.url, this.connectionParams}) {
    _streamController
      ..onListen = () {
        _connectIfNeeded().ignore();
        _stateNotifier.addListener(_reconnectIfNeeded);
      }
      ..onCancel = () async {
        _stateNotifier.removeListener(_reconnectIfNeeded);
        _reconnectTimer?.cancel();
        _reconnectTimer = null;

        final state = _stateNotifier.value;
        _stateNotifier.value = const WsState.disconnected();

        if (state case _WsConnectedState(:final ws)) {
          ws.close().ignore();
        }
      };
  }

  final String url;
  final FutureOr<Map<String, dynamic>> Function()? connectionParams;

  Timer? _reconnectTimer;
  final ValueNotifier<WsState> _stateNotifier = ValueNotifier(const WsState.disconnected());
  final StreamController<WsMessage> _streamController = StreamController<WsMessage>.broadcast();

  final RequestSerializer serializer = const RequestSerializer();
  final ResponseParser parser = const ResponseParser();
  final Uuid uuid = const Uuid();

  @override
  Stream<Response> request(Request request, [NextLink? forward]) {
    final subscriptionId = uuid.v4();
    StreamSubscription<WsMessage>? subscription;

    void stateNotifierListener() {
      final state = _stateNotifier.value;
      if (state case _WsConnectedState(:final ws)) {
        ws.sendMessage(WsMessage.subscribe(id: subscriptionId, payload: serializer.serializeRequest(request)));
      }
    }

    final controller = StreamController<Response>.broadcast();
    controller
      ..onListen = () async {
        subscription = _streamController.stream.listen((message) async {
          switch (message) {
            case _WsNextMessage(:final id, :final payload) when id == subscriptionId:
              controller.add(parser.parseResponse(payload));
            case _WsErrorMessage(:final id, :final payload) when id == subscriptionId:
              payload.map(parser.parseError).forEach(controller.addError);
              await subscription?.cancel();
              subscription = null;
            case _WsCompleteMessage(:final id) when id == subscriptionId:
              await subscription?.cancel();
              subscription = null;
            default:
              break;
          }
        });

        stateNotifierListener();
        _stateNotifier.addListener(stateNotifierListener);
      }
      ..onCancel = () async {
        _stateNotifier.removeListener(stateNotifierListener);

        final state = _stateNotifier.value;
        if (state case _WsConnectedState(:final ws)) {
          ws.sendMessage(WsMessage.complete(id: subscriptionId));
        }

        await subscription?.cancel();
        subscription = null;
      };

    return controller.stream;
  }

  Future<void> _connectIfNeeded() async {
    final state = _stateNotifier.value;
    if (state is! _WsDisconnectedState || !_streamController.hasListener) {
      return;
    }

    _stateNotifier.value = const WsState.connecting();
    final completer = Completer<void>();

    try {
      final ws = await WebSocket.connect(Uri.parse(url), protocols: ['graphql-transport-ws']);

      void handleMessage(WsMessage message) {
        switch (message) {
          case _WsPingMessage(:final payload):
            ws.sendMessage(WsMessage.pong(payload: payload));
          case _WsPongMessage():
            break;
          case _WsConnectionAckMessage() when !completer.isCompleted:
            completer.complete();
          case _ when !completer.isCompleted:
            completer.completeError(Exception('First message cannot be $message'));
          case _WsNextMessage():
          case _WsErrorMessage():
          case _WsCompleteMessage():
            _streamController.add(message);
          default:
            break;
        }
      }

      ws.events.listen((event) {
        switch (event) {
          case TextDataReceived(:final text):
            final message = WsMessage.fromJson(json.decode(text) as Map<String, dynamic>);
            handleMessage(message);
          case CloseReceived():
            if (completer.isCompleted) {
              _stateNotifier.value = const WsState.disconnected();
            } else {
              completer.completeError(Exception('WebSocket closed'));
            }
          default:
            break;
        }
      });

      final payload = await connectionParams?.call();
      ws.sendMessage(WsMessage.connectionInit(payload: payload));

      await completer.future;

      _stateNotifier.value = WsState.connected(ws: ws);
    } on Exception {
      _stateNotifier.value = const WsState.disconnected();
    }
  }

  void _reconnectIfNeeded() {
    if (_reconnectTimer != null) {
      return;
    }

    final state = _stateNotifier.value;
    if (state is _WsDisconnectedState && _streamController.hasListener) {
      _reconnectTimer = Timer(const Duration(seconds: 1), () {
        _reconnectTimer = null;
        _connectIfNeeded().ignore();
      });
    }
  }

  @override
  Future<void> dispose() async {
    _reconnectTimer?.cancel();
    _reconnectTimer = null;

    final state = _stateNotifier.value;
    if (state case _WsConnectedState(:final ws)) {
      await ws.close(1000, 'Normal Closure');
    }

    _stateNotifier.dispose();
    await _streamController.close();
  }
}

@freezed
sealed class WsState with _$WsState {
  const factory WsState.connected({required WebSocket ws}) = _WsConnectedState;
  const factory WsState.connecting() = _WsConnectingState;
  const factory WsState.disconnected() = _WsDisconnectedState;
}

@Freezed(unionKey: 'type', unionValueCase: FreezedUnionCase.snake)
sealed class WsMessage with _$WsMessage {
  const factory WsMessage.connectionInit({Map<String, dynamic>? payload}) = _WsConnectionInitMessage;
  const factory WsMessage.connectionAck() = _WsConnectionAckMessage;
  const factory WsMessage.subscribe({required String id, required Map<String, dynamic> payload}) = _WsSubscribeMessage;
  const factory WsMessage.next({required String id, required Map<String, dynamic> payload}) = _WsNextMessage;
  const factory WsMessage.error({required String id, required List<Map<String, dynamic>> payload}) = _WsErrorMessage;
  const factory WsMessage.complete({required String id}) = _WsCompleteMessage;
  const factory WsMessage.ping({Map<String, dynamic>? payload}) = _WsPingMessage;
  const factory WsMessage.pong({Map<String, dynamic>? payload}) = _WsPongMessage;

  factory WsMessage.fromJson(Map<String, dynamic> json) => _$WsMessageFromJson(json);
}

extension on WebSocket {
  void sendMessage(WsMessage message) {
    sendText(json.encode(message));
  }
}
