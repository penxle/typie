enum SelectionHandleType { from, to }

class SelectionHandleInfo {
  const SelectionHandleInfo({required this.pageIdx, required this.x, required this.y, required this.height});

  factory SelectionHandleInfo.fromMap(Map<String, dynamic> map) {
    final bounds = map['bounds'] as Map<String, dynamic>?;
    return SelectionHandleInfo(
      pageIdx: map['pageIdx'] as int,
      x: (bounds?['x'] as num?)?.toDouble() ?? 0,
      y: (bounds?['y'] as num?)?.toDouble() ?? 0,
      height: (bounds?['height'] as num?)?.toDouble() ?? 0,
    );
  }

  final int pageIdx;
  final double x;
  final double y;
  final double height;
}
