import 'dart:typed_data';

import 'package:freezed_annotation/freezed_annotation.dart';

part 'models.freezed.dart';
part 'models.g.dart';

@Freezed(unionKey: 'type')
sealed class ExternalElementData with _$ExternalElementData {
  const factory ExternalElementData.image({String? id, required double proportion, String? uploadId}) =
      ImageElementData;

  const factory ExternalElementData.file({String? id, String? uploadId}) = FileElementData;

  const factory ExternalElementData.embed({String? id}) = EmbedElementData;

  const factory ExternalElementData.archived({String? id}) = ArchivedElementData;

  factory ExternalElementData.fromJson(Map<String, dynamic> json) => _$ExternalElementDataFromJson(json);
}

@freezed
abstract class ExternalElementBounds with _$ExternalElementBounds {
  const factory ExternalElementBounds({
    required double x,
    required double y,
    required double width,
    required double height,
  }) = _ExternalElementBounds;

  factory ExternalElementBounds.fromJson(Map<String, dynamic> json) => _$ExternalElementBoundsFromJson(json);
}

@freezed
abstract class ExternalElement with _$ExternalElement {
  const factory ExternalElement({
    required int pageIdx,
    required String nodeId,
    required ExternalElementBounds bounds,
    required ExternalElementData data,
    required bool isSelected,
  }) = _ExternalElement;

  factory ExternalElement.fromJson(Map<String, dynamic> json) => _$ExternalElementFromJson(json);
}

class InflightImage {
  const InflightImage({required this.bytes, required this.width, required this.height});

  final Uint8List bytes;
  final int width;
  final int height;
}

class InflightFile {
  const InflightFile({required this.path, required this.name, required this.size});

  final String path;
  final String name;
  final int size;
}

@freezed
abstract class ImageAsset with _$ImageAsset {
  const factory ImageAsset({
    required String id,
    required String url,
    required int width,
    required int height,
    required double ratio,
    String? placeholder,
  }) = _ImageAsset;

  factory ImageAsset.fromJson(Map<String, dynamic> json) => _$ImageAssetFromJson(json);
}

@freezed
abstract class FileAsset with _$FileAsset {
  const factory FileAsset({required String id, required String url, required String name, required int size}) =
      _FileAsset;

  factory FileAsset.fromJson(Map<String, dynamic> json) => _$FileAssetFromJson(json);
}

@freezed
abstract class EmbedAsset with _$EmbedAsset {
  const factory EmbedAsset({
    required String id,
    required String url,
    String? title,
    String? description,
    String? thumbnailUrl,
    String? html,
  }) = _EmbedAsset;

  factory EmbedAsset.fromJson(Map<String, dynamic> json) => _$EmbedAssetFromJson(json);
}

@freezed
abstract class ArchivedAsset with _$ArchivedAsset {
  const factory ArchivedAsset({required String id, required String content}) = _ArchivedAsset;
  factory ArchivedAsset.fromJson(Map<String, dynamic> json) => _$ArchivedAssetFromJson(json);
}
