import 'dart:async';
import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:back_button_interceptor/back_button_interceptor.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/error.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/__generated__/create_note_mutation.req.gql.dart';
import 'package:typie/screens/editor/__generated__/delete_note_mutation.req.gql.dart';
import 'package:typie/screens/editor/__generated__/move_note_mutation.req.gql.dart';
import 'package:typie/screens/editor/__generated__/post_related_notes_query.data.gql.dart';
import 'package:typie/screens/editor/__generated__/post_related_notes_query.req.gql.dart';
import 'package:typie/screens/editor/__generated__/update_note_mutation.req.gql.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/values.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

class Note extends HookWidget {
  const Note({super.key, required this.onBack});

  final void Function() onBack;

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final data = useValueListenable(scope.data);
    final entityId = data?.post.entity.id;

    if (entityId == null) {
      return const AppErrorWidget();
    }

    final refreshNotifier = useMemoized(RefreshNotifier.new, []);

    return GraphQLOperation(
      operation: GPostRelatedNotesScreen_QueryReq((b) => b..vars.entityId = entityId),
      refreshNotifier: refreshNotifier,
      builder: (context, client, queryData) {
        final notes = queryData.entity.notes.toList();
        final sortedNotes = List<GPostRelatedNotesScreen_QueryData_entity_notes>.from(notes)
          ..sort((a, b) => a.order.compareTo(b.order));

        return _NoteContent(
          sortedNotes: sortedNotes,
          entityId: entityId,
          onBack: onBack,
          refreshNotifier: refreshNotifier,
        );
      },
    );
  }
}

class _NoteContent extends HookWidget {
  const _NoteContent({
    required this.sortedNotes,
    required this.entityId,
    required this.onBack,
    required this.refreshNotifier,
  });

  final List<GPostRelatedNotesScreen_QueryData_entity_notes> sortedNotes;
  final String entityId;
  final void Function() onBack;
  final RefreshNotifier refreshNotifier;

  @override
  Widget build(BuildContext context) {
    useAutomaticKeepAlive();

    final scope = EditorStateScope.of(context);
    final mode = useValueListenable(scope.mode);
    final client = useService<GraphQLClient>();
    final mixpanel = useService<Mixpanel>();

    final noteControllers = useState<Map<String, TextEditingController>>({});
    final noteLocalUpdatedAt = useState<Map<String, DateTime>>({});
    final focusedNoteId = useRef<String?>(null);
    final debounceTimers = useRef<Map<String, Timer>>({});
    final focusNodes = useState<Map<String, FocusNode>>({});

    final focusedNoteIdState = useState<String?>(null);

    useEffect(() {
      final newControllers = Map<String, TextEditingController>.from(noteControllers.value);
      final newFocusNodes = Map<String, FocusNode>.from(focusNodes.value);
      final newLocalUpdatedAt = Map<String, DateTime>.from(noteLocalUpdatedAt.value);

      for (final note in sortedNotes) {
        if (!newControllers.containsKey(note.id)) {
          newControllers[note.id] = TextEditingController(text: note.content);
          final focusNode = FocusNode();
          focusNode.addListener(() {
            if (focusNode.hasFocus) {
              focusedNoteIdState.value = note.id;
            } else if (focusedNoteIdState.value == note.id) {
              focusedNoteIdState.value = null;
            }
          });
          newFocusNodes[note.id] = focusNode;
        } else {
          final controller = newControllers[note.id]!;
          final updatedAt = DateTime.parse(note.updatedAt.toString());
          if (newLocalUpdatedAt[note.id] == null || updatedAt.isAfter(newLocalUpdatedAt[note.id]!)) {
            if (controller.text != note.content) {
              controller.text = note.content;
            }
            newLocalUpdatedAt[note.id] = updatedAt;
          }
        }
      }

      final currentIds = sortedNotes.map((n) => n.id).toSet();
      newControllers.removeWhere((id, controller) {
        if (!currentIds.contains(id)) {
          controller.dispose();
          return true;
        }
        return false;
      });
      newFocusNodes.removeWhere((id, focusNode) {
        if (!currentIds.contains(id)) {
          focusNode.dispose();
          return true;
        }
        return false;
      });
      debounceTimers.value.removeWhere((id, timer) {
        if (!currentIds.contains(id)) {
          timer.cancel();
          return true;
        }
        return false;
      });
      newLocalUpdatedAt.removeWhere((id, _) => !currentIds.contains(id));

      noteControllers.value = newControllers;
      focusNodes.value = newFocusNodes;
      noteLocalUpdatedAt.value = newLocalUpdatedAt;

      return null;
    }, [sortedNotes]);

    useEffect(() {
      if (mode != EditorMode.note) {
        return null;
      }

      bool handler(bool stopDefaultButtonEvent, RouteInfo routeInfo) {
        onBack();
        return true;
      }

      BackButtonInterceptor.add(handler);

      return () {
        BackButtonInterceptor.remove(handler);
      };
    }, [mode]);

    useEffect(() {
      return () {
        for (final timer in debounceTimers.value.values) {
          timer.cancel();
        }
        for (final controller in noteControllers.value.values) {
          controller.dispose();
        }
        for (final focusNode in focusNodes.value.values) {
          focusNode.dispose();
        }
      };
    }, []);

    List<Map<String, dynamic>> getNoteColors() {
      final backgroundColors = editorValues['textBackgroundColor']!;
      return backgroundColors.where((item) => item['value'] != 'none').toList();
    }

    String getRandomNoteColor() {
      final colors = getNoteColors();
      final random = Random();
      return colors[random.nextInt(colors.length)]['value'] as String;
    }

    Color getNoteBackgroundColor(String? color) {
      final backgroundColors = editorValues['textBackgroundColor']!;

      final colorMap = backgroundColors.firstWhere((item) => item['value'] == color, orElse: () => {'color': null});

      final colorFunc = colorMap['color'] as Color Function(BuildContext)?;
      if (colorFunc != null) {
        return colorFunc(context);
      }

      return context.colors.prosemirrorWhite;
    }

    Future<void> handleAddNote() async {
      final randomColor = getRandomNoteColor();

      final request = GPostRelatedNotesScreen_CreateNote_MutationReq(
        (b) => b
          ..vars.input.entityId = Value.present(entityId)
          ..vars.input.content = ''
          ..vars.input.color = randomColor,
      );

      final response = await client.request(request);
      final newNoteId = response.createNote.id;
      focusedNoteId.value = newNoteId;

      unawaited(mixpanel.track('create_related_note', properties: {'via': 'button'}));
      refreshNotifier.refresh();
    }

    Future<void> handleUpdateNote(String noteId, String content) async {
      noteLocalUpdatedAt.value[noteId] = DateTime.now();

      debounceTimers.value[noteId]?.cancel();
      debounceTimers.value[noteId] = Timer(const Duration(milliseconds: 500), () async {
        final request = GPostRelatedNotesScreen_UpdateNote_MutationReq(
          (b) => b
            ..vars.input.noteId = noteId
            ..vars.input.content = Value.present(content),
        );

        await client.request(request);
      });
    }

    Future<void> handleDeleteNote(String noteId) async {
      await context.showModal(
        child: ConfirmModal(
          title: '노트 삭제',
          message: '이 노트를 삭제하시겠어요? \n복구할 수 없어요.',
          confirmText: '삭제',
          confirmBackgroundColor: context.colors.accentDanger,
          onConfirm: () async {
            debounceTimers.value.remove(noteId)?.cancel();
            noteLocalUpdatedAt.value.remove(noteId);

            final request = GPostRelatedNotesScreen_DeleteNote_MutationReq((b) => b..vars.input.noteId = noteId);

            await client.request(request);

            unawaited(mixpanel.track('delete_related_note'));
            refreshNotifier.refresh();
          },
        ),
      );
    }

    // NOTE: 노트 추가 후 포커스
    useEffect(() {
      if (focusedNoteId.value != null && focusNodes.value.containsKey(focusedNoteId.value)) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          focusNodes.value[focusedNoteId.value]?.requestFocus();
          focusedNoteId.value = null;
        });
      }
      return null;
    }, [focusedNoteId.value, focusNodes.value.keys.toList()]);

    return Screen(
      resizeToAvoidBottomInset: true,
      heading: Heading(
        leadingWidget: Tappable(
          onTap: onBack,
          padding: const Pad(vertical: 4),
          child: SizedBox(width: 52, child: Icon(LucideLightIcons.chevron_left, color: context.colors.textDefault)),
        ),
        titleIcon: LucideLightIcons.sticky_note,
        title: '이 포스트 관련 노트',
        backgroundColor: context.colors.surfaceDefault,
        actions: [HeadingAction(icon: LucideLightIcons.plus, onTap: handleAddNote)],
      ),
      backgroundColor: context.colors.surfaceDefault,
      child: sortedNotes.isEmpty
          ? Center(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Container(
                    width: 64,
                    height: 64,
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(16),
                      color: context.colors.surfaceMuted,
                    ),
                    child: Icon(LucideLightIcons.sticky_note, color: context.colors.textFaint, size: 28),
                  ),
                  const SizedBox(height: 20),
                  Text(
                    '떠오르는 생각이나 아이디어를\n자유롭게 기록해보세요.\n\n글쓰기 중 상단바를 쓸어넘겨서 이 포스트 관련 노트를 볼 수 있어요.',
                    textAlign: TextAlign.center,
                    style: TextStyle(fontSize: 16, color: context.colors.textFaint),
                  ),
                  const SizedBox(height: 20),
                  Tappable(
                    onTap: handleAddNote,
                    child: Container(
                      decoration: BoxDecoration(
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(color: context.colors.borderStrong),
                      ),
                      padding: const Pad(vertical: 12, horizontal: 20),
                      child: Text('노트 추가', style: TextStyle(fontSize: 16, color: context.colors.textDefault)),
                    ),
                  ),
                ],
              ),
            )
          : ReorderableListView.builder(
              padding: Pad(horizontal: 20, top: 12, bottom: MediaQuery.viewPaddingOf(context).bottom + 20),
              itemCount: sortedNotes.length,
              buildDefaultDragHandles: false,
              proxyDecorator: (child, index, animation) => child,
              onReorderStart: (index) async {
                await HapticFeedback.lightImpact();
              },
              onReorderEnd: (index) async {
                await HapticFeedback.lightImpact();
              },
              onReorder: (oldIndex, newIndex) async {
                if (oldIndex < newIndex) {
                  newIndex -= 1;
                }

                final movedNoteId = sortedNotes[oldIndex].id;

                final movedNote = sortedNotes.removeAt(oldIndex);
                sortedNotes.insert(newIndex, movedNote);

                final lowerNote = newIndex > 0 ? sortedNotes[newIndex - 1] : null;
                final upperNote = newIndex < sortedNotes.length - 1 ? sortedNotes[newIndex + 1] : null;

                final request = GPostRelatedNotesScreen_MoveNote_MutationReq(
                  (b) => b
                    ..vars.input.noteId = movedNoteId
                    ..vars.input.lowerOrder = Value.present(lowerNote?.order)
                    ..vars.input.upperOrder = Value.present(upperNote?.order),
                );

                await client.request(request);

                unawaited(mixpanel.track('move_related_note'));
                refreshNotifier.refresh();
              },
              itemBuilder: (context, index) {
                final note = sortedNotes[index];
                final controller = noteControllers.value[note.id];
                final focusNode = focusNodes.value[note.id];

                return Padding(
                  key: ValueKey(note.id),
                  padding: const Pad(bottom: 12),
                  child: Stack(
                    children: [
                      ClipPath(
                        clipper: _NoteFoldClipper(),
                        child: Material(
                          color: getNoteBackgroundColor(note.color),
                          child: IntrinsicHeight(
                            child: Row(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                ReorderableDragStartListener(
                                  index: index,
                                  child: Container(
                                    width: 32,
                                    padding: const Pad(horizontal: 8, vertical: 12),
                                    child: Icon(
                                      LucideLightIcons.grip_vertical,
                                      color: context.colors.textFaint,
                                      size: 16,
                                    ),
                                  ),
                                ),
                                Expanded(
                                  child: Padding(
                                    padding: const Pad(top: 12, right: 12, bottom: 12),
                                    child: TextField(
                                      controller: controller,
                                      focusNode: focusNode,
                                      smartDashesType: SmartDashesType.disabled,
                                      smartQuotesType: SmartQuotesType.disabled,
                                      autocorrect: false,
                                      keyboardType: TextInputType.multiline,
                                      maxLines: null,
                                      minLines: 3,
                                      decoration: InputDecoration.collapsed(
                                        hintText: '기억할 내용이나 작성에 도움이 되는 내용을 자유롭게 적어보세요.',
                                        hintStyle: TextStyle(fontSize: 14, color: context.colors.textDisabled),
                                      ),
                                      style: TextStyle(fontSize: 14, color: context.colors.textDefault),
                                      onChanged: (value) {
                                        unawaited(handleUpdateNote(note.id, value));
                                      },
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ),
                      ),
                      if (focusedNoteIdState.value == note.id)
                        Positioned(
                          bottom: 12,
                          right: 12,
                          child: Tappable(
                            onTap: () => handleDeleteNote(note.id),
                            child: Container(
                              padding: const Pad(all: 4),
                              decoration: BoxDecoration(
                                borderRadius: BorderRadius.circular(4),
                                color: Colors.transparent,
                              ),
                              child: Icon(LucideLightIcons.trash_2, color: context.colors.textDefault, size: 16),
                            ),
                          ),
                        ),
                      Positioned(
                        bottom: 0,
                        right: 0,
                        child: ClipPath(
                          clipper: _TriangleClipper(),
                          child: Container(
                            width: 12,
                            height: 12,
                            decoration: BoxDecoration(
                              gradient: LinearGradient(
                                begin: Alignment.topLeft,
                                end: Alignment.bottomRight,
                                colors: [Colors.black.withValues(alpha: 0.05), Colors.black.withValues(alpha: 0.15)],
                              ),
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                );
              },
            ),
    );
  }
}

class _NoteFoldClipper extends CustomClipper<Path> {
  @override
  Path getClip(Size size) {
    final path = Path();
    const foldSize = 12.0;

    path
      ..moveTo(0, 0)
      ..lineTo(size.width, 0)
      ..lineTo(size.width, size.height - foldSize)
      ..lineTo(size.width - foldSize, size.height)
      ..lineTo(0, size.height)
      ..close();

    return path;
  }

  @override
  bool shouldReclip(CustomClipper<Path> oldClipper) => false;
}

class _TriangleClipper extends CustomClipper<Path> {
  @override
  Path getClip(Size size) {
    final path = Path()
      ..moveTo(0, 0)
      ..lineTo(size.width, 0)
      ..lineTo(0, size.height)
      ..close();
    return path;
  }

  @override
  bool shouldReclip(CustomClipper<Path> oldClipper) => false;
}
