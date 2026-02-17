import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:typie/env.dart';
import 'package:typie/service.dart';

final _cache = <String, String?>{};
final _loading = <String>{};

class FontSpecimen extends HookWidget {
  const FontSpecimen({required this.text, this.fontId, required this.style, super.key});

  final String text;
  final String? fontId;
  final TextStyle style;

  @override
  Widget build(BuildContext context) {
    final rebuild = useState(0);

    final id = fontId;
    final cacheKey = id != null ? '$id:$text' : null;

    useEffect(() {
      if (cacheKey == null || _cache.containsKey(cacheKey) || _loading.contains(cacheKey)) {
        return null;
      }

      _loading.add(cacheKey);

      unawaited(() async {
        try {
          final response = await serviceLocator<Dio>().get<String>(
            '${Env.apiUrl}/font/$id/specimen',
            queryParameters: {'text': text},
          );
          _cache[cacheKey] = response.data;
        } on DioException catch (e) {
          if (e.response?.statusCode == 422) {
            _cache[cacheKey] = null;
          }
        } finally {
          _loading.remove(cacheKey);
          rebuild.value++;
        }
      }());

      return null;
    }, [cacheKey]);

    final svg = cacheKey != null ? _cache[cacheKey] : null;

    if (svg != null) {
      final mergedStyle = DefaultTextStyle.of(context).style.merge(style);
      final fontSize = mergedStyle.fontSize!;
      final lineHeight = mergedStyle.height!;

      return SizedBox(
        height: fontSize * lineHeight,
        child: Center(
          child: SvgPicture.string(
            svg,
            height: fontSize,
            theme: mergedStyle.color != null ? SvgTheme(currentColor: mergedStyle.color!) : null,
            clipBehavior: Clip.antiAlias,
          ),
        ),
      );
    }

    return Text(text, style: style);
  }
}
