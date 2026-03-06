import 'dart:async';
import 'dart:convert';
import 'dart:developer' as dev;
import 'dart:io';
import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:device_info_plus/device_info_plus.dart';
import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart' show InputBorder, InputDecoration, TextField;
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:gap/gap.dart';
import 'package:jiffy/jiffy.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/controller/input.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/widgets/tappable.dart';

const _sentinel = kDebugMode ? '◆' : '\u200B';
const _recordingEndpoint = 'https://input-recorder-worker.penxle.workers.dev';

class EditorTextInput extends StatefulWidget {
  const EditorTextInput({required this.brightness, required this.controller, super.key});

  final Brightness brightness;
  final InputController controller;

  @override
  State<EditorTextInput> createState() => EditorTextInputState();
}

class EditorTextInputState extends State<EditorTextInput> with DeltaTextInputClient, WidgetsBindingObserver {
  TextInputConnection? _connection;
  TextEditingValue _currentValue = TextEditingValue.empty;
  bool _controlled = false;
  bool _sentinelLost = false;
  late final FocusNode _focusNode;

  InputController get _controller => widget.controller;

  final List<Map<String, dynamic>> _recordingEntries = [];
  List<Map<String, dynamic>>? _collectingDispatches;

  void _startCollectingDispatches() {
    _collectingDispatches = [];
    _controller.onDispatchRecorded = (msg) {
      _collectingDispatches?.add(Map<String, dynamic>.from(msg));
    };
  }

  List<Map<String, dynamic>> _stopCollectingDispatches() {
    final result = _collectingDispatches ?? [];
    _collectingDispatches = null;
    _controller.onDispatchRecorded = null;
    return result;
  }

  void _addRecordingEntry(Map<String, dynamic> entry) {
    _recordingEntries.add(entry);
    if (_recordingEntries.length > 100) {
      _recordingEntries.removeRange(0, _recordingEntries.length - 100);
    }
  }

  static Map<String, dynamic> _serializeValue(TextEditingValue value) => {
    'text': value.text.replaceAll('\u200B', '◆'),
    'selection': {'baseOffset': value.selection.baseOffset, 'extentOffset': value.selection.extentOffset},
    'composing': {'start': value.composing.start, 'end': value.composing.end},
  };

  static Map<String, dynamic> _serializeDelta(TextEditingDelta delta) {
    final base = {
      'oldText': delta.oldText.replaceAll('\u200B', '◆'),
      'selection': {'baseOffset': delta.selection.baseOffset, 'extentOffset': delta.selection.extentOffset},
      'composing': {'start': delta.composing.start, 'end': delta.composing.end},
    };

    return switch (delta) {
      TextEditingDeltaInsertion() => {
        ...base,
        'type': 'insertion',
        'textInserted': delta.textInserted,
        'insertionOffset': delta.insertionOffset,
      },
      TextEditingDeltaDeletion() => {
        ...base,
        'type': 'deletion',
        'deletedRange': {'start': delta.deletedRange.start, 'end': delta.deletedRange.end},
      },
      TextEditingDeltaReplacement() => {
        ...base,
        'type': 'replacement',
        'replacedRange': {'start': delta.replacedRange.start, 'end': delta.replacedRange.end},
        'replacementText': delta.replacementText,
      },
      TextEditingDeltaNonTextUpdate() => {...base, 'type': 'nonTextUpdate'},
      _ => {...base, 'type': 'unknown'},
    };
  }

  Future<Map<String, dynamic>> _collectDeviceInfo() async {
    final deviceInfo = await DeviceInfoPlugin().deviceInfo;

    final (os, model) = switch (deviceInfo) {
      IosDeviceInfo() => (
        '${deviceInfo.systemName} ${deviceInfo.systemVersion}',
        '${deviceInfo.modelName} (${deviceInfo.model})',
      ),
      AndroidDeviceInfo() => (
        'Android ${deviceInfo.version.release}',
        '${deviceInfo.manufacturer} ${deviceInfo.model} (${deviceInfo.brand})',
      ),
      _ => throw UnimplementedError(),
    };

    final keyboard = Platform.isAndroid ? await Keyboard.getCurrentKeyboard() : null;

    return {'os': os, 'model': model, 'keyboard': keyboard};
  }

  Future<void> _sendRecording(String name) async {
    final device = await _collectDeviceInfo();
    final payload = jsonEncode({
      'name': name,
      'timestamp': Jiffy.now().toUtc().format(),
      'device': device,
      'entries': List<Map<String, dynamic>>.from(_recordingEntries),
    });

    try {
      await Dio().post<void>(
        _recordingEndpoint,
        data: payload,
        options: Options(headers: {'Content-Type': 'application/json'}),
      );
    } catch (err) {
      dev.log('[InputRecording] Failed to send: $err');
    }
  }

  void showRecordingSheet() {
    if (!mounted) {
      return;
    }
    unawaited(
      context.showBottomSheet(child: _InputRecordingBottomSheet(onSend: (name) => unawaited(_sendRecording(name)))),
    );
  }

  TextInputConfiguration get _configuration => TextInputConfiguration(
    inputType: TextInputType.multiline,
    inputAction: TextInputAction.newline,
    enableDeltaModel: true,
    smartDashesType: SmartDashesType.disabled,
    smartQuotesType: SmartQuotesType.disabled,
    keyboardAppearance: widget.brightness,
  );

  @override
  void initState() {
    super.initState();
    _focusNode = FocusNode();
    WidgetsBinding.instance.addObserver(this);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _controller.onInputReady();
    });
  }

  @override
  void didUpdateWidget(covariant EditorTextInput oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.brightness != widget.brightness && _connection != null && _connection!.attached) {
      _connection!.updateConfig(_configuration);
    }
  }

  @override
  void dispose() {
    _focusNode.dispose();
    _connection?.close();
    _connection = null;
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed && _connection != null && _connection!.attached) {
      _connection!.show();
    }
  }

  @override
  TextEditingValue get currentTextEditingValue => _currentValue;

  @override
  AutofillScope? get currentAutofillScope => null;

  @override
  void updateEditingValueWithDeltas(List<TextEditingDelta> deltas) {
    final oldValue = _currentValue;
    final newValue = deltas.fold(_currentValue, (value, delta) => delta.apply(value));

    if (oldValue.text.startsWith(_sentinel) && !newValue.text.startsWith(_sentinel)) {
      _sentinelLost = true;
    }

    final serializedDeltas = <Map<String, dynamic>>[];
    var deltaCurrent = oldValue;
    for (final delta in deltas) {
      final deltaBefore = deltaCurrent;
      final deltaAfter = delta.apply(deltaCurrent);
      final isInit =
          delta is TextEditingDeltaNonTextUpdate &&
          (deltaBefore.selection.baseOffset < 0 ||
              deltaBefore.selection.extentOffset < 0 ||
              deltaAfter.selection.baseOffset < 0 ||
              deltaAfter.selection.extentOffset < 0);
      if (!isInit) {
        serializedDeltas.add({
          'before': _serializeValue(deltaBefore),
          'after': _serializeValue(deltaAfter),
          'delta': _serializeDelta(delta),
        });
      }
      deltaCurrent = deltaAfter;
    }

    _startCollectingDispatches();

    final hadComposing = oldValue.composing.isValid && !oldValue.composing.isCollapsed;
    final hasComposing = newValue.composing.isValid && !newValue.composing.isCollapsed;

    if (hadComposing || hasComposing) {
      if (hasComposing) {
        final text = newValue.text.substring(newValue.composing.start, newValue.composing.end);
        final replaceLength = (!hadComposing && oldValue.text == newValue.text) ? text.length : 0;
        _controlled = true;
        _controller.compositionUpdate(text, replaceLength: replaceLength);
      } else {
        if (oldValue.text != newValue.text) {
          final prefix = oldValue.text.substring(0, oldValue.composing.start);
          final suffix = oldValue.text.substring(oldValue.composing.end);
          final committed = newValue.text.substring(prefix.length, newValue.text.length - suffix.length);
          final hasNewline = committed.contains('\n');
          final committedText = hasNewline ? committed.replaceAll('\n', '') : committed;

          if (committedText.isEmpty) {
            _controller.compositionEnd();
          } else {
            _controlled = false;
            _controller
              ..compositionUpdate(committedText)
              ..commitPreedit();
          }

          if (hasNewline) {
            _controlled = false;
            _controller.insertNewline();
            final dispatches = _stopCollectingDispatches();

            final revertText = prefix + committedText + suffix;
            _setCurrentValue(
              TextEditingValue(
                text: revertText,
                selection: TextSelection.collapsed(offset: prefix.length + committedText.length),
              ),
            );

            if (_connection != null && _connection!.attached) {
              _connection!.setEditingState(_currentValue);
            }

            if (serializedDeltas.isNotEmpty) {
              _addRecordingEntry({
                'type': 'batch',
                'before': _serializeValue(oldValue),
                'after': _serializeValue(_currentValue),
                'deltas': serializedDeltas,
                'dispatches': dispatches,
              });
            }
            return;
          }
        } else {
          _controller.commitPreedit();
        }
      }
    } else if (newValue.text.isEmpty) {
      final removedLen = oldValue.selection.baseOffset;
      _controlled = false;
      _controller.onDeleteBackward(length: removedLen);
    } else if (!deltas.every((delta) => delta is TextEditingDeltaNonTextUpdate)) {
      final effectiveOld = oldValue.text.startsWith(_sentinel) ? oldValue.text.substring(1) : oldValue.text;
      final effectiveNew = newValue.text.startsWith(_sentinel) ? newValue.text.substring(1) : newValue.text;

      if (effectiveOld != effectiveNew) {
        final minLen = effectiveOld.length < effectiveNew.length ? effectiveOld.length : effectiveNew.length;
        var commonPrefix = 0;
        while (commonPrefix < minLen && effectiveOld[commonPrefix] == effectiveNew[commonPrefix]) {
          commonPrefix++;
        }

        final oldSentinelLen = oldValue.text.length - effectiveOld.length;
        final newSentinelLen = newValue.text.length - effectiveNew.length;
        final oldCursor = (oldValue.selection.baseOffset - oldSentinelLen).clamp(commonPrefix, effectiveOld.length);
        final newCursor = (newValue.selection.baseOffset - newSentinelLen).clamp(commonPrefix, effectiveNew.length);

        final removedLen = oldCursor - commonPrefix;
        final insertedText = effectiveNew.substring(commonPrefix, newCursor);

        if (insertedText.contains('\n')) {
          _controlled = false;
          _controller.insertNewline();
          final dispatches = _stopCollectingDispatches();
          _setCurrentValue(oldValue);

          if (_connection != null && _connection!.attached) {
            _connection!.setEditingState(_currentValue);
          }

          if (serializedDeltas.isNotEmpty) {
            _addRecordingEntry({
              'type': 'batch',
              'before': _serializeValue(oldValue),
              'after': _serializeValue(_currentValue),
              'deltas': serializedDeltas,
              'dispatches': dispatches,
            });
          }
          return;
        }

        if (removedLen > 0 && insertedText.isNotEmpty) {
          _controlled = true;
          _controller.onReplaceBackward(removedLen, insertedText);
        } else if (removedLen > 0) {
          _controlled = false;
          _controller.onDeleteBackward(length: removedLen);
        } else if (insertedText.isNotEmpty) {
          _controlled = true;
          _controller.onInsertText(insertedText);
        }
      }
    } else if (oldValue.selection.isCollapsed && newValue.selection.isCollapsed) {
      final delta = max(newValue.selection.baseOffset, 1) - oldValue.selection.baseOffset;
      if (delta != 0) {
        for (var i = 0; i < delta.abs(); i++) {
          _controlled = false;
          _controller.navigate(delta > 0 ? 'right' : 'left');
        }

        if (newValue.selection.baseOffset == 0) {
          final dispatches = _stopCollectingDispatches();
          _setCurrentValue(TextEditingValue(text: newValue.text, selection: const TextSelection.collapsed(offset: 1)));
          if (_connection != null && _connection!.attached) {
            _connection!.setEditingState(_currentValue);
          }
          if (serializedDeltas.isNotEmpty) {
            _addRecordingEntry({
              'type': 'batch',
              'before': _serializeValue(oldValue),
              'after': _serializeValue(_currentValue),
              'deltas': serializedDeltas,
              'dispatches': dispatches,
            });
          }
          return;
        }
      }
    }

    final dispatches = _stopCollectingDispatches();
    if (serializedDeltas.isNotEmpty) {
      _addRecordingEntry({
        'type': 'batch',
        'before': _serializeValue(oldValue),
        'after': _serializeValue(newValue),
        'deltas': serializedDeltas,
        'dispatches': dispatches,
      });
    }

    _setCurrentValue(newValue);

    if (_currentValue.text != newValue.text && _connection != null && _connection!.attached) {
      _connection!.setEditingState(_currentValue);
    }
  }

  @override
  void updateFloatingCursor(RawFloatingCursorPoint point) {
    _controlled = false;

    switch (point.state) {
      case FloatingCursorDragState.Start:
        _controller.onFloatingCursorBegin();
      case FloatingCursorDragState.Update:
        final offset = point.offset;
        if (offset != null) {
          _controller.onFloatingCursorUpdate(offset.dx, offset.dy);
        }
      case FloatingCursorDragState.End:
        _controller.onFloatingCursorEnd();
    }
  }

  @override
  void performAction(TextInputAction action) {}

  @override
  void performSelector(String selectorName) {}

  @override
  void connectionClosed() {
    _connection = null;
    _controller.onFocusLost();
  }

  @override
  void showAutocorrectionPromptRect(int start, int end) {}

  @override
  void insertTextPlaceholder(Size size) {}

  @override
  void removeTextPlaceholder() {}

  @override
  void didChangeInputControl(TextInputControl? oldControl, TextInputControl? newControl) {}

  @override
  void insertContent(KeyboardInsertedContent content) {}

  @override
  void performPrivateCommand(String action, Map<String, dynamic> data) {}

  @override
  void showToolbar() {}

  @override
  void updateEditingValue(TextEditingValue value) {}

  KeyEventResult _handleKeyEvent(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }

    if (_currentValue.composing.isValid && !_currentValue.composing.isCollapsed) {
      _addRecordingEntry({
        'type': 'keyEvent',
        'key': event.logicalKey.keyLabel,
        'before': _serializeValue(_currentValue),
        'after': _serializeValue(_currentValue),
        'dispatches': <Map<String, dynamic>>[],
      });
      return KeyEventResult.ignored;
    }

    final value = _currentValue;
    final sel = value.selection;
    final minOffset = value.text.startsWith(_sentinel) ? 1 : 0;

    TextEditingValue? newValue;

    if (event.logicalKey == LogicalKeyboardKey.backspace) {
      var deleteLength = 0;

      if (sel.isCollapsed) {
        if (sel.baseOffset > minOffset) {
          deleteLength = 1;
          newValue = TextEditingValue(
            text: value.text.substring(0, sel.baseOffset - 1) + value.text.substring(sel.baseOffset),
            selection: TextSelection.collapsed(offset: sel.baseOffset - 1),
          );
        } else if (minOffset > 0) {
          deleteLength = 1;
        }
      } else {
        final start = sel.start.clamp(minOffset, value.text.length);
        final end = sel.end.clamp(minOffset, value.text.length);
        deleteLength = end - start;
        newValue = TextEditingValue(
          text: value.text.substring(0, start) + value.text.substring(end),
          selection: TextSelection.collapsed(offset: start),
        );
      }

      if (newValue != null || deleteLength > 0) {
        _startCollectingDispatches();
        if (newValue != null) {
          _setCurrentValue(newValue);
          if (_connection != null && _connection!.attached) {
            _connection!.setEditingState(newValue);
          }
        }
        _controlled = false;
        _controller.onDeleteBackward(length: deleteLength);
        final dispatches = _stopCollectingDispatches();
        _addRecordingEntry({
          'type': 'keyEvent',
          'key': event.logicalKey.keyLabel,
          'before': _serializeValue(value),
          'after': _serializeValue(_currentValue),
          'dispatches': dispatches,
        });
      }

      return KeyEventResult.handled;
    }

    _addRecordingEntry({
      'type': 'keyEvent',
      'key': event.logicalKey.keyLabel,
      'before': _serializeValue(value),
      'after': _serializeValue(value),
      'dispatches': <Map<String, dynamic>>[],
    });

    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    return Focus(focusNode: _focusNode, onKeyEvent: _handleKeyEvent, child: const SizedBox.shrink());
  }

  void activateInput() {
    FocusManager.instance.primaryFocus?.unfocus();

    if (_connection == null || !_connection!.attached) {
      _connection = TextInput.attach(this, _configuration);
      _connection!.setEditingState(_currentValue);
    }

    _connection!.show();
    _focusNode.requestFocus();
  }

  void deactivateInput() {
    _controlled = false;
    _connection?.close();
    _connection = null;
  }

  void updateCursor(double x, double y, double height) {
    if (_connection?.attached ?? false) {
      _connection!.setCaretRect(Rect.fromLTWH(x, y, 1, height));
    }
  }

  void invalidate() {
    _controlled = false;
  }

  bool reconcile(String nodeId, String precedingText, String followingText) {
    final newValue = TextEditingValue(
      text: precedingText + followingText,
      selection: TextSelection.collapsed(offset: precedingText.length),
    );

    if (_currentValue != newValue) {
      if (_controlled) {
        return false;
      }

      _setCurrentValue(
        TextEditingValue(
          text: precedingText + followingText,
          selection: TextSelection.collapsed(offset: precedingText.length),
        ),
      );

      if (_connection != null && _connection!.attached) {
        _connection!.setEditingState(_currentValue);
      }
    }

    return true;
  }

  void _setCurrentValue(TextEditingValue value) {
    if (_currentValue == value) {
      return;
    }

    final needsSentinel =
        !value.text.startsWith(_sentinel) &&
        (_sentinelLost && !(value.composing.isValid && !value.composing.isCollapsed) ||
            value.selection == const TextSelection.collapsed(offset: 0));

    if (needsSentinel) {
      _currentValue = value.copyWith(
        text: _sentinel + value.text,
        selection: value.selection.baseOffset >= 0
            ? TextSelection(baseOffset: value.selection.baseOffset + 1, extentOffset: value.selection.extentOffset + 1)
            : value.selection,
        composing: value.composing.isValid
            ? TextRange(start: value.composing.start + 1, end: value.composing.end + 1)
            : value.composing,
      );
      _sentinelLost = false;
    } else {
      _currentValue = value;
    }
  }
}

class _InputRecordingBottomSheet extends StatefulWidget {
  const _InputRecordingBottomSheet({required this.onSend});

  final void Function(String name) onSend;

  @override
  State<_InputRecordingBottomSheet> createState() => _InputRecordingBottomSheetState();
}

class _InputRecordingBottomSheetState extends State<_InputRecordingBottomSheet> {
  final _nameController = TextEditingController();

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Text('입력 기록 전송', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
          const Gap(8),
          Text(
            '이 글의 최근 입력 기록이 개발자에게 분석 목적으로 전송돼요. 제품 개선 외의 목적으로는 사용되지 않아요.',
            style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
          ),
          const Gap(12),
          Container(
            decoration: BoxDecoration(
              border: Border.all(color: context.colors.borderDefault),
              borderRadius: BorderRadius.circular(8),
            ),
            child: TextField(
              controller: _nameController,
              style: TextStyle(fontSize: 16, color: context.colors.textDefault),
              cursorColor: context.colors.textDefault,
              decoration: InputDecoration(
                hintText: '설명을 입력하세요',
                hintStyle: TextStyle(fontSize: 16, color: context.colors.textFaint),
                border: InputBorder.none,
                contentPadding: const Pad(horizontal: 12, vertical: 12),
              ),
              autofocus: true,
            ),
          ),
          const Gap(24),
          Row(
            spacing: 8,
            children: [
              Expanded(
                child: Tappable(
                  onTap: () async {
                    await context.router.maybePop();
                  },
                  child: Container(
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      color: context.colors.surfaceMuted,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    padding: const Pad(vertical: 16),
                    child: const Text('취소', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
                  ),
                ),
              ),
              Expanded(
                child: Tappable(
                  onTap: () async {
                    widget.onSend(_nameController.text.trim());
                    if (context.mounted) {
                      await context.router.maybePop();
                    }
                  },
                  child: Container(
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      color: context.colors.surfaceInverse,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    padding: const Pad(vertical: 16),
                    child: Text(
                      '보내기',
                      style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600, color: context.colors.textInverse),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
