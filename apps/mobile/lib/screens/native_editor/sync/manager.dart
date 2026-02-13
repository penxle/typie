import 'dart:async';
import 'dart:convert';

import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:flutter/foundation.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/__generated__/document_sync_stream.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/document_sync_stream.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/sync_document.req.gql.dart';
import 'package:typie/screens/native_editor/context.dart';
import 'package:typie/screens/native_editor/sync/persistence.dart';
import 'package:uuid/uuid.dart';

enum ConnectionStatus { connecting, connected, disconnected }

class SyncManager {
  SyncManager({
    required this.documentId,
    required this.editor,
    required this.client,
    required LocalPersistence persistence,
    required EditorContext editorContext,
  }) : _persistence = persistence,
       _editorContext = editorContext;

  final String documentId;
  final NativeEditor editor;
  final GraphQLClient client;
  final LocalPersistence _persistence;
  final EditorContext _editorContext;

  static const _disconnectThreshold = Duration(seconds: 3);

  final String _clientId = const Uuid().v4();
  final ValueNotifier<ConnectionStatus> connectionStatus = ValueNotifier(ConnectionStatus.connecting);

  DateTime _lastHeartbeatAt = DateTime.now();
  Timer? _syncUpdateTimer;
  Timer? _forceSyncTimer;
  Timer? _fullSyncTimer;
  Timer? _heartbeatTimer;
  StreamSubscription<dynamic>? _subscription;
  StreamSubscription<List<ConnectivityResult>>? _connectivitySubscription;

  bool _disposed = false;

  Future<void> start() async {
    final connectivityResult = await Connectivity().checkConnectivity();
    if (connectivityResult.contains(ConnectivityResult.none)) {
      connectionStatus.value = ConnectionStatus.disconnected;
    }

    _connectivitySubscription = Connectivity().onConnectivityChanged.listen((results) {
      if (_disposed) {
        return;
      }
      if (results.contains(ConnectivityResult.none)) {
        connectionStatus.value = ConnectionStatus.disconnected;
      } else {
        final isFresh = DateTime.now().difference(_lastHeartbeatAt) <= _disconnectThreshold;
        connectionStatus.value = isFresh ? ConnectionStatus.connected : ConnectionStatus.connecting;
      }
    });

    _heartbeatTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (_disposed) {
        return;
      }
      if (DateTime.now().difference(_lastHeartbeatAt) > _disconnectThreshold) {
        connectionStatus.value = ConnectionStatus.disconnected;
      }
    });

    await fullSync();

    _subscription = client
        .subscribe(
          GNativeEditor_DocumentSyncStream_SubscriptionReq(
            (b) => b
              ..vars.clientId = _clientId
              ..vars.documentId = documentId,
          ),
        )
        .listen(_handleSyncMessage);

    _forceSyncTimer = Timer.periodic(const Duration(seconds: 10), (_) => forceSync());
    _fullSyncTimer = Timer.periodic(const Duration(seconds: 60), (_) => fullSync());
  }

  void handleDocChanged() {
    if (_disposed) {
      return;
    }

    if (_persistence.version.isNotEmpty) {
      final updates = editor.export(DocExportMode.updatesFrom, _persistence.version);
      if (updates != null) {
        unawaited(_persistence.saveUpdate(updates));
      }
    }

    _syncUpdateTimer?.cancel();
    _syncUpdateTimer = Timer(const Duration(seconds: 1), _sendUpdates);
  }

  Future<void> _sendUpdates() async {
    if (_disposed) {
      return;
    }

    final updates = _persistence.checkpoint.isNotEmpty
        ? editor.export(DocExportMode.updatesFrom, _persistence.checkpoint)
        : null;
    if (updates == null || updates.isEmpty) {
      return;
    }

    try {
      await _doSync(type: GDocumentSyncType.UPDATE, data: base64Encode(updates));
    } catch (err) {
      debugPrint('Sync error: $err');
    }
  }

  Future<void> fullSync() async {
    if (_disposed) {
      return;
    }

    final snapshot = editor.export(DocExportMode.snapshot);
    final version = editor.export(DocExportMode.version);

    if (snapshot != null && version != null) {
      await _persistence.saveSnapshot(snapshot, Uint8List.fromList(version));
    }

    if (_disposed) {
      return;
    }

    final update = editor.export(DocExportMode.allUpdates);
    if (update != null && update.isNotEmpty) {
      try {
        await _doSync(type: GDocumentSyncType.UPDATE, data: base64Encode(update));
      } catch (err) {
        debugPrint('Full sync error: $err');
      }
    }
  }

  Future<void> forceSync() async {
    if (_disposed) {
      return;
    }

    final version = editor.export(DocExportMode.version);
    if (version == null) {
      return;
    }

    try {
      await _doSync(type: GDocumentSyncType.VECTOR, data: base64Encode(version));
    } catch (err) {
      debugPrint('Force sync error: $err');
    }
  }

  Future<void> _doSync({required GDocumentSyncType type, required String data}) async {
    final result = await client.request(
      GNativeEditor_SyncDocument_MutationReq(
        (b) => b
          ..vars.input.clientId = _clientId
          ..vars.input.documentId = documentId
          ..vars.input.type = type
          ..vars.input.data = data,
      ),
    );

    for (final payload in result.syncDocument) {
      await _handleSyncPayload(payload.type, payload.data);
    }
  }

  Future<void> _handleSyncPayload(GDocumentSyncType type, String data) async {
    switch (type) {
      case GDocumentSyncType.HEARTBEAT:
        _lastHeartbeatAt = DateTime.parse(data);
        connectionStatus.value = ConnectionStatus.connected;
      case GDocumentSyncType.UPDATE:
        editor.importUpdates(Uint8List.fromList(base64Decode(data)));
      case GDocumentSyncType.VECTOR:
        await _persistence.saveCheckpoint(Uint8List.fromList(base64Decode(data)));
      case GDocumentSyncType.RESET:
        await _persistence.clear();
        _editorContext.serverSnapshot = Uint8List.fromList(base64Decode(data));
        _editorContext.resetKey.value++;
      default:
        break;
    }
  }

  void _handleSyncMessage(GNativeEditor_DocumentSyncStream_SubscriptionData data) {
    if (_disposed) {
      return;
    }

    final payload = data.documentSyncStream;
    unawaited(_handleSyncPayload(payload.type, payload.data));
  }

  void dispose() {
    _disposed = true;
    _syncUpdateTimer?.cancel();
    _forceSyncTimer?.cancel();
    _fullSyncTimer?.cancel();
    _heartbeatTimer?.cancel();
    unawaited(_subscription?.cancel());
    unawaited(_connectivitySubscription?.cancel());

    if (_persistence.checkpoint.isNotEmpty) {
      final updates = editor.export(DocExportMode.updatesFrom, _persistence.checkpoint);
      if (updates != null && updates.isNotEmpty) {
        unawaited(
          client.request(
            GNativeEditor_SyncDocument_MutationReq(
              (b) => b
                ..vars.input.clientId = _clientId
                ..vars.input.documentId = documentId
                ..vars.input.type = GDocumentSyncType.UPDATE
                ..vars.input.data = base64Encode(updates),
            ),
          ),
        );
      }
    }

    _persistence.dispose();
    connectionStatus.dispose();
  }
}
