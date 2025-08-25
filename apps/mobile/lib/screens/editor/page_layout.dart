import 'package:typie/screens/editor/schema.dart';

const double minContentSizeMm = 50;

const double minPageSizeMm = 100;
const double maxPageSizeMm = 2000;

const Map<String, Map<String, double>> pageSizeMap = {
  'a4': {'width': 210, 'height': 297},
  'a5': {'width': 148, 'height': 210},
  'b5': {'width': 176, 'height': 250},
  'b6': {'width': 125, 'height': 176},
};

const Map<String, Map<String, double>> defaultPageMargins = {
  'a4': {'top': 25, 'bottom': 25, 'left': 25, 'right': 25},
  'a5': {'top': 20, 'bottom': 20, 'left': 20, 'right': 20},
  'b5': {'top': 15, 'bottom': 15, 'left': 15, 'right': 15},
  'b6': {'top': 10, 'bottom': 10, 'left': 10, 'right': 10},
};

const List<Map<String, String>> pageLayoutOptions = [
  {'label': 'A4 (210mm × 297mm)', 'value': 'a4'},
  {'label': 'A5 (148mm × 210mm)', 'value': 'a5'},
  {'label': 'B5 (176mm × 250mm)', 'value': 'b5'},
  {'label': 'B6 (125mm × 176mm)', 'value': 'b6'},
  {'label': '직접 지정', 'value': 'custom'},
];

PageLayout createDefaultPageLayout([String preset = 'a4']) {
  final size = pageSizeMap[preset]!;
  final margins = defaultPageMargins[preset]!;

  return PageLayout(
    width: size['width']!,
    height: size['height']!,
    marginTop: margins['top']!,
    marginBottom: margins['bottom']!,
    marginLeft: margins['left']!,
    marginRight: margins['right']!,
  );
}

double getMaxMargin(String side, PageLayout pageLayout) {
  switch (side) {
    case 'left':
      return (pageLayout.width - pageLayout.marginRight - minContentSizeMm).clamp(0, double.infinity);
    case 'right':
      return (pageLayout.width - pageLayout.marginLeft - minContentSizeMm).clamp(0, double.infinity);
    case 'top':
      return (pageLayout.height - pageLayout.marginBottom - minContentSizeMm).clamp(0, double.infinity);
    case 'bottom':
      return (pageLayout.height - pageLayout.marginTop - minContentSizeMm).clamp(0, double.infinity);
    default:
      return 0;
  }
}
