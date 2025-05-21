import 'package:flutter/material.dart';

extension IterableExtension<T> on Iterable<T> {
  Iterable<T> intersperseWith(T element) sync* {
    final iterator = this.iterator;
    if (iterator.moveNext()) {
      yield iterator.current;
      while (iterator.moveNext()) {
        yield element;
        yield iterator.current;
      }
    }
  }
}

extension IterableWidgetExtension<T extends Widget> on Iterable<T> {
  Iterable<Widget> intersperseWith(Widget element) sync* {
    final iterator = this.iterator;
    if (iterator.moveNext()) {
      yield iterator.current;
      while (iterator.moveNext()) {
        yield element;
        yield iterator.current;
      }
    }
  }
}
