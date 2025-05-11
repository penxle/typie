import 'dart:async';

import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';

extension LoaderExtension on BuildContext {
  static OverlayEntry? _entry;

  Future<T> runWithLoader<T>(Future<T> Function() fn) async {
    if (_entry != null) {
      _entry!.remove();
    }

    _entry = OverlayEntry(
      builder: (context) {
        return Stack(
          children: [
            ModalBarrier(dismissible: false, color: AppColors.black.withValues(alpha: 0.5)),
            const Center(child: CircularProgressIndicator(color: AppColors.gray_950)),
          ],
        );
      },
    );

    Overlay.of(this, rootOverlay: true).insert(_entry!);

    try {
      return await fn();
    } finally {
      _entry!.remove();
      _entry = null;
    }
  }
}
