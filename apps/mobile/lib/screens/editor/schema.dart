import 'package:collection/collection.dart';
import 'package:freezed_annotation/freezed_annotation.dart';

part 'schema.freezed.dart';
part 'schema.g.dart';

@freezed
abstract class ProseMirrorState with _$ProseMirrorState {
  const factory ProseMirrorState({
    required List<ProseMirrorNode> nodes,
    required List<ProseMirrorMark> marks,
    required ProseMirrorSelection selection,
    List<ProseMirrorMark>? storedMarks,
  }) = _ProseMirrorState;

  const ProseMirrorState._();
  factory ProseMirrorState.fromJson(Map<String, dynamic> json) => _$ProseMirrorStateFromJson(json);

  ProseMirrorNode? get currentNode => selection is ProseMirrorNodeSelection ? nodes.last : null;

  bool isMarkActive(String type, {Map<String, dynamic>? attrs}) {
    if (storedMarks != null) {
      final storedMark = storedMarks?.firstWhereOrNull((mark) => mark.type == type);
      if (storedMark == null) {
        return false;
      }

      if (attrs == null || attrs.isEmpty) {
        return true;
      }

      for (final entry in attrs.entries) {
        if (!storedMark.attrs!.containsKey(entry.key) || storedMark.attrs![entry.key] != entry.value) {
          return false;
        }
      }

      return true;
    }

    final mark = marks.firstWhereOrNull((mark) => mark.type == type);
    if (mark == null) {
      return false;
    }

    if (attrs == null || attrs.isEmpty) {
      return true;
    }

    if (mark.attrs == null) {
      return false;
    }

    for (final entry in attrs.entries) {
      if (!mark.attrs!.containsKey(entry.key) || mark.attrs![entry.key] != entry.value) {
        return false;
      }
    }

    return true;
  }

  Map<String, dynamic>? getMarkAttributes(String type) {
    if (storedMarks != null) {
      final storedMark = storedMarks?.firstWhereOrNull((mark) => mark.type == type);
      if (storedMark == null) {
        return null;
      }

      return storedMark.attrs;
    }

    final mark = marks.firstWhereOrNull((mark) => mark.type == type);
    if (mark == null) {
      return null;
    }

    return mark.attrs;
  }

  bool isNodeActive(String type, {Map<String, dynamic>? attrs}) {
    final node = nodes.firstWhereOrNull((node) => node.type == type);
    if (node == null) {
      return false;
    }

    if (attrs == null || attrs.isEmpty) {
      return true;
    }

    if (node.attrs == null) {
      return false;
    }

    for (final entry in attrs.entries) {
      if (!node.attrs!.containsKey(entry.key) || node.attrs![entry.key] != entry.value) {
        return false;
      }
    }

    return true;
  }

  Map<String, dynamic>? getNodeAttributes(String type) {
    final node = nodes.firstWhereOrNull((node) => node.type == type);
    if (node == null) {
      return null;
    }

    return node.attrs;
  }
}

@freezed
abstract class ProseMirrorMark with _$ProseMirrorMark {
  const factory ProseMirrorMark({required String type, required Map<String, dynamic>? attrs}) = _ProseMirrorMark;
  factory ProseMirrorMark.fromJson(Map<String, dynamic> json) => _$ProseMirrorMarkFromJson(json);
}

@freezed
abstract class ProseMirrorNode with _$ProseMirrorNode {
  const factory ProseMirrorNode({required int pos, required String type, required Map<String, dynamic>? attrs}) =
      _ProseMirrorNode;
  factory ProseMirrorNode.fromJson(Map<String, dynamic> json) => _$ProseMirrorNodeFromJson(json);
}

@Freezed(unionKey: 'type')
sealed class ProseMirrorSelection with _$ProseMirrorSelection {
  const factory ProseMirrorSelection.all() = ProseMirrorAllSelection;
  const factory ProseMirrorSelection.text({required int anchor, required int head}) = ProseMirrorTextSelection;
  const factory ProseMirrorSelection.node({required int anchor}) = ProseMirrorNodeSelection;
  const factory ProseMirrorSelection.multinode({required int anchor, required int head}) =
      ProseMirrorMultiNodeSelection;
  const factory ProseMirrorSelection.cell({required int anchor, required int head}) = ProseMirrorCellSelection;

  factory ProseMirrorSelection.fromJson(Map<String, dynamic> json) => _$ProseMirrorSelectionFromJson(json);
}

@freezed
abstract class CharacterCountState with _$CharacterCountState {
  const factory CharacterCountState({
    required int countWithWhitespace,
    required int countWithoutWhitespace,
    required int countWithoutWhitespaceAndPunctuation,
  }) = _CharacterCountState;

  factory CharacterCountState.fromJson(Map<String, dynamic> json) => _$CharacterCountStateFromJson(json);
}

@freezed
abstract class YJSState with _$YJSState {
  const factory YJSState({required int maxWidth}) = _YJSState;

  factory YJSState.fromJson(Map<String, dynamic> json) => _$YJSStateFromJson(json);
}
