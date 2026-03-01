import 'dart:io';
import 'dart:ui' as ui;

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/controller/upload.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/floating.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:uuid/uuid.dart';

class NativeEditorImageFloatingToolbar extends StatelessWidget {
  const NativeEditorImageFloatingToolbar({required this.element, super.key});

  final ExternalElement element;

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final uploadManager = scope.uploadManager;
    final imageData = element.data as ImageElementData;

    final localUploadId = uploadManager.localImageUploadIds[element.nodeId];
    final currentUploadId = imageData.uploadId ?? localUploadId;
    final hasImage = imageData.id != null || currentUploadId != null;

    return Row(
      spacing: 8,
      children: [
        if (!hasImage)
          FloatingToolbarButton(
            icon: LucideLightIcons.image,
            onTap: () => _handleUpload(scope, uploadManager, context),
          ),
        FloatingToolbarButton(
          icon: LucideLightIcons.trash_2,
          onTap: () {
            uploadManager.removeLocalImageUploadId(element.nodeId);
            scope.dispatch({'type': 'deleteNode', 'nodeId': element.nodeId});
            scope.controller.scrollIntoView();
          },
        ),
      ],
    );
  }

  Future<void> _handleUpload(NativeEditorToolbarScope scope, UploadManager uploadManager, BuildContext context) async {
    final result = await FilePicker.platform.pickFiles(type: FileType.image, allowMultiple: true).catchError((err) {
      if (context.mounted) {
        context.toast(ToastType.error, '이미지를 선택할 수 없습니다');
      }
      return null;
    });

    if (result == null || result.files.isEmpty) {
      return;
    }

    final platformFiles = result.files.where((f) => f.path != null).toList();
    if (platformFiles.isEmpty) {
      return;
    }

    const uuid = Uuid();
    final imageData = element.data as ImageElementData;

    final firstFile = platformFiles.first;
    final firstUploadId = imageData.uploadId ?? uuid.v4();

    if (imageData.uploadId == null) {
      uploadManager.setLocalImageUploadId(element.nodeId, firstUploadId);
    }

    try {
      final file = File(firstFile.path!);
      final bytes = await file.readAsBytes();
      final codec = await ui.instantiateImageCodec(bytes);
      final frame = await codec.getNextFrame();
      final width = frame.image.width;
      final height = frame.image.height;

      uploadManager.addInflightImage(firstUploadId, InflightImage(bytes: bytes, width: width, height: height));
    } catch (err) {
      uploadManager.removeLocalImageUploadId(element.nodeId);
      if (context.mounted) {
        context.toast(ToastType.error, '이미지를 불러올 수 없습니다');
      }
      return;
    }

    final restFiles = platformFiles.sublist(1);
    for (final platformFile in restFiles) {
      final uploadId = uuid.v4();
      try {
        final file = File(platformFile.path!);
        final bytes = await file.readAsBytes();
        final codec = await ui.instantiateImageCodec(bytes);
        final frame = await codec.getNextFrame();
        final width = frame.image.width;
        final height = frame.image.height;

        uploadManager.addInflightImage(uploadId, InflightImage(bytes: bytes, width: width, height: height));
        scope.dispatch({'type': 'insertImage', 'uploadId': uploadId});
        scope.controller.scrollIntoView();
      } catch (err) {
        uploadManager.removeInflightImage(uploadId);
      }
    }
  }
}
