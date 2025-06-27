import 'package:flutter/material.dart';
import 'package:typie/styles/semantic_colors.dart';

extension BuildContextExtensions on BuildContext {
  SemanticColors get colors => Theme.of(this).extension<SemanticColors>()!;
}
