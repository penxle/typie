import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/search/__generated__/screen.data.gql.dart';
import 'package:typie/screens/search/__generated__/screen.req.gql.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class SearchScreen extends HookWidget {
  const SearchScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final pref = useService<Pref>();
    final client = useService<GraphQLClient>();

    final result = useState<List<GSearchScreen_QueryData_search_hits>>([]);

    return Screen(
      heading: Heading(
        title: '검색',
        titleWidget: TextField(
          autocorrect: false,
          textAlignVertical: TextAlignVertical.top,
          keyboardType: TextInputType.text,
          smartDashesType: SmartDashesType.disabled,
          decoration: const InputDecoration(
            hintText: '검색어를 입력하세요',
            hintStyle: TextStyle(color: AppColors.gray_500, fontWeight: FontWeight.w500),
            fillColor: AppColors.gray_200,
            filled: true,
            border: InputBorder.none,
          ),
          onChanged: (value) async {
            final res = await client.request(
              GSearchScreen_QueryReq(
                (b) => b
                  ..vars.siteId = pref.siteId
                  ..vars.query = value,
              ),
            );
            result.value = res.search.hits.toList();
          },
        ),
        titleIcon: LucideLightIcons.search,
      ),
      padding: const Pad(all: 20),
      child: GraphQLOperation(
        operation: GSearchScreen_QueryReq(
          (b) => b
            ..vars.siteId = pref.siteId
            ..vars.query = '',
        ),
        builder: (context, client, data) {
          return result.value.isEmpty
              ? const Center(
                  child: Text('검색어를 입력해주세요', style: TextStyle(fontSize: 15, color: AppColors.gray_700)),
                )
              : ListView.builder(
                  itemCount: result.value.length,
                  itemBuilder: (context, index) {
                    return ListTile(
                      contentPadding: Pad.zero,
                      title: result.value[index].when(
                        searchHitPost: (post) => Tappable(
                          onTap: () async {
                            await context.router.push(EditorRoute(slug: post.post.entity.slug));
                          },
                          child: Container(
                            decoration: BoxDecoration(
                              border: Border.all(color: AppColors.gray_950),
                              borderRadius: BorderRadius.circular(8),
                              color: AppColors.white,
                            ),
                            padding: const Pad(all: 16),
                            child: Row(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment: CrossAxisAlignment.start,
                                    spacing: 4,
                                    children: [
                                      _HTMLText(
                                        post.title ?? '(제목 없음)',
                                        style: const TextStyle(
                                          fontSize: 16,
                                          fontWeight: FontWeight.w500,
                                          color: AppColors.gray_950,
                                        ),
                                      ),
                                      _HTMLText(
                                        post.text ?? '(내용 없음)',
                                        style: const TextStyle(
                                          fontSize: 16,
                                          fontWeight: FontWeight.w500,
                                          color: AppColors.gray_950,
                                        ),
                                      ),
                                    ],
                                  ),
                                ),
                                Text(
                                  post.post.updatedAt.fromNow(),
                                  style: const TextStyle(fontSize: 14, color: AppColors.gray_700),
                                ),
                              ],
                            ),
                          ),
                        ),
                        orElse: () => throw UnimplementedError(),
                      ),
                    );
                  },
                );
        },
      ),
    );
  }
}

class _HTMLText extends StatelessWidget {
  const _HTMLText(this.text, {this.style = const TextStyle(color: AppColors.gray_950)});

  final String text;
  final TextStyle style;

  TextSpan _buildTextSpan(String input) {
    final emRegExp = RegExp('<em>(.*?)</em>');
    final spans = <InlineSpan>[];
    var currentIndex = 0;

    for (final match in emRegExp.allMatches(input)) {
      if (match.start > currentIndex) {
        spans.add(TextSpan(text: input.substring(currentIndex, match.start), style: style));
      }

      spans.add(
        TextSpan(
          text: match.group(1),
          style: style.copyWith(fontWeight: FontWeight.w700, color: AppColors.gray_950),
        ),
      );

      currentIndex = match.end;
    }

    if (currentIndex < input.length) {
      spans.add(TextSpan(text: input.substring(currentIndex), style: style));
    }

    return TextSpan(children: spans);
  }

  @override
  Widget build(BuildContext context) {
    return Text.rich(_buildTextSpan(text), maxLines: 1, overflow: TextOverflow.ellipsis);
  }
}
