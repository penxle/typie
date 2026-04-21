import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

class VisibleEditorArea {
  const VisibleEditorArea({required this.viewportSize, required this.topInset, required this.bottomInset});

  final Size viewportSize;
  final double topInset;
  final double bottomInset;

  double get width => viewportSize.width;
  double get height => viewportSize.height;

  double get top {
    return topInset.clamp(0.0, height);
  }

  double get bottom {
    final inset = bottomInset.clamp(0.0, height);
    return (height - inset).clamp(top, height);
  }

  Offset get origin => Offset(0, top);

  Size get size => Size(width, visibleHeight);

  Rect get bounds => Rect.fromLTRB(0, top, width, bottom);

  double get visibleHeight => bounds.height;

  Offset localToVisible(Offset localOffset) => localOffset - origin;

  Offset visibleToLocal(Offset visibleOffset) => visibleOffset + origin;

  double localToVisibleY(double localY) => localY - top;

  double visibleToLocalY(double visibleY) => visibleY + top;

  @override
  bool operator ==(Object other) {
    return other is VisibleEditorArea &&
        other.viewportSize == viewportSize &&
        other.topInset == topInset &&
        other.bottomInset == bottomInset;
  }

  @override
  int get hashCode => Object.hash(viewportSize, topInset, bottomInset);
}

class VisibleEditorAreaNotifier extends ValueNotifier<VisibleEditorArea> {
  VisibleEditorAreaNotifier({
    required ValueListenable<Size> viewportSize,
    required ValueListenable<double> topInset,
    required ValueListenable<double> bottomInset,
  }) : _viewportSize = viewportSize,
       _topInset = topInset,
       _bottomInset = bottomInset,
       super(
         VisibleEditorArea(viewportSize: viewportSize.value, topInset: topInset.value, bottomInset: bottomInset.value),
       ) {
    _viewportSize.addListener(_syncValue);
    _topInset.addListener(_syncValue);
    _bottomInset.addListener(_syncValue);
  }

  final ValueListenable<Size> _viewportSize;
  final ValueListenable<double> _topInset;
  final ValueListenable<double> _bottomInset;

  void _syncValue() {
    final nextValue = VisibleEditorArea(
      viewportSize: _viewportSize.value,
      topInset: _topInset.value,
      bottomInset: _bottomInset.value,
    );
    if (value == nextValue) {
      return;
    }
    value = nextValue;
  }

  @override
  void dispose() {
    _viewportSize.removeListener(_syncValue);
    _topInset.removeListener(_syncValue);
    _bottomInset.removeListener(_syncValue);
    super.dispose();
  }
}
