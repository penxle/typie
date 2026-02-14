import 'dart:typed_data';

import 'package:hive_ce/hive.dart';
import 'package:path_provider/path_provider.dart';

const _boxName = 'documents';

class _StoredDocument {
  _StoredDocument({
    required this.id,
    required this.snapshot,
    required this.updates,
    required this.version,
    required this.checkpoint,
    required this.updatedAt,
    this.generation = 0,
  });

  final String id;
  final Uint8List? snapshot;
  final List<Uint8List> updates;
  final Uint8List version;
  final Uint8List checkpoint;
  final int updatedAt;
  final int generation;

  _StoredDocument withUpdatedAt({
    Uint8List? snapshot,
    List<Uint8List>? updates,
    Uint8List? version,
    Uint8List? checkpoint,
    int? generation,
  }) {
    return _StoredDocument(
      id: id,
      snapshot: snapshot ?? this.snapshot,
      updates: updates ?? this.updates,
      version: version ?? this.version,
      checkpoint: checkpoint ?? this.checkpoint,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
      generation: generation ?? this.generation,
    );
  }

  Map<String, dynamic> toMap() {
    return {
      'id': id,
      'snapshot': snapshot,
      'updates': updates,
      'version': version,
      'checkpoint': checkpoint,
      'updatedAt': updatedAt,
      'generation': generation,
    };
  }

  static _StoredDocument? fromMap(Map<dynamic, dynamic>? map) {
    if (map == null) {
      return null;
    }

    return _StoredDocument(
      id: map['id'] as String,
      snapshot: map['snapshot'] as Uint8List?,
      updates: (map['updates'] as List?)?.cast<Uint8List>() ?? [],
      version: map['version'] as Uint8List? ?? Uint8List(0),
      checkpoint: map['checkpoint'] as Uint8List? ?? Uint8List(0),
      updatedAt: map['updatedAt'] as int,
      generation: map['generation'] as int? ?? 0,
    );
  }
}

class LocalPersistence {
  LocalPersistence(this.documentId);

  final String documentId;
  Box<dynamic>? _box;
  bool _disposed = false;
  Uint8List _version = Uint8List(0);
  Uint8List _checkpoint = Uint8List(0);
  int _generation = 0;

  Uint8List get version => _version;
  Uint8List get checkpoint => _checkpoint;
  int get generation => _generation;

  Future<void> _ensureBox() async {
    if (_box != null) {
      return;
    }

    final dir = await getApplicationDocumentsDirectory();
    final path = '${dir.path}/hive';
    Hive.init(path);

    _box = await Hive.openBox(_boxName);
  }

  Future<({Uint8List? snapshot, List<Uint8List> updates})?> load() async {
    if (_disposed) {
      return null;
    }

    await _ensureBox();
    final data = _box!.get(documentId);
    final doc = _StoredDocument.fromMap(data as Map<dynamic, dynamic>?);

    if (doc == null) {
      return null;
    }

    _version = doc.version;
    _checkpoint = doc.checkpoint;
    _generation = doc.generation;

    return (snapshot: doc.snapshot, updates: doc.updates);
  }

  Future<void> saveUpdate(Uint8List update) async {
    if (_disposed) {
      return;
    }

    await _ensureBox();
    final existing = _StoredDocument.fromMap(_box!.get(documentId) as Map<dynamic, dynamic>?);

    if (existing != null) {
      await _put(existing.withUpdatedAt(updates: [...existing.updates, update]));
    }
  }

  Future<void> saveSnapshot(Uint8List snapshot, Uint8List version, {int? generation}) async {
    if (_disposed) {
      return;
    }

    await _ensureBox();
    await _put(
      _StoredDocument(
        id: documentId,
        snapshot: snapshot,
        updates: [],
        version: version,
        checkpoint: _checkpoint,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
        generation: generation ?? _generation,
      ),
    );
    _version = version;
    if (generation != null) {
      _generation = generation;
    }
  }

  Future<void> saveCheckpoint(Uint8List checkpoint) async {
    _checkpoint = checkpoint;

    if (_disposed) {
      return;
    }

    await _ensureBox();
    final existing = _StoredDocument.fromMap(_box!.get(documentId) as Map<dynamic, dynamic>?);
    if (existing == null) {
      return;
    }

    await _put(existing.withUpdatedAt(checkpoint: checkpoint));
  }

  Future<void> _put(_StoredDocument doc) async {
    await _box!.put(documentId, doc.toMap());
  }

  Future<void> clear() async {
    if (_disposed) {
      return;
    }

    await _ensureBox();
    await _box!.delete(documentId);
    _version = Uint8List(0);
    _checkpoint = Uint8List(0);
  }

  void dispose() {
    _disposed = true;
  }
}
