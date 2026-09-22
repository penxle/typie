import Testing

@testable import Design

@MainActor
@Suite struct TDialogCenterTests {
  nonisolated private static let failureTitle = "로그인할 수 없어요"
  nonisolated private static let failureMessage = "오류가 발생했어요. 잠시 후 다시 시도해주세요."
  nonisolated private static let confirmText = "확인"
  nonisolated private static let retryTitle = "문제가 발생했어요"
  nonisolated private static let retryMessage = "잠시 후 다시 시도해주세요."
  nonisolated private static let retryText = "다시 시도"
  nonisolated private static let removeTitle = "일일 목표를 해제하시겠어요?"
  nonisolated private static let removeMessage = "설정한 하루 목표 글자 수가 사라져요."
  nonisolated private static let removeText = "해제"
  nonisolated private static let cancelText = "취소"

  private static func failure() -> TDialogItem {
    TDialogItem(title: failureTitle, message: failureMessage, confirmText: confirmText)
  }

  private static func retry() -> TDialogItem {
    TDialogItem(title: retryTitle, message: retryMessage, confirmText: retryText)
  }

  private static func remove() -> TDialogItem {
    TDialogItem(
      title: removeTitle, message: removeMessage, confirmText: removeText, cancelText: cancelText,
      confirmIsDestructive: true)
  }

  @Test func startsEmpty() {
    #expect(TDialogCenter().current == nil)
  }

  @Test func presentPublishesItem() {
    let center = TDialogCenter()
    let item = Self.failure()

    center.present(item)

    #expect(center.current == item)
  }

  @Test func dismissClearsWithoutInvokingCallback() {
    let center = TDialogCenter()
    let recorder = DismissRecorder()

    center.present(Self.failure()) { [weak center] in
      recorder.record(cleared: center?.current == nil)
    }
    center.dismiss()

    #expect(center.current == nil)
    #expect(recorder.count == 0)
  }

  @Test func confirmClearsBeforeInvokingCallbackOnce() {
    let center = TDialogCenter()
    let recorder = DismissRecorder()

    center.present(Self.failure()) { [weak center] in
      recorder.record(cleared: center?.current == nil)
    }
    center.confirm()

    #expect(center.current == nil)
    #expect(recorder.count == 1)
    #expect(recorder.clearedWhenCalled == true)

    center.confirm()

    #expect(recorder.count == 1)
  }

  @Test func repeatedPresentKeepsLastItemAndCallback() {
    let center = TDialogCenter()
    let first = DismissRecorder()
    let second = DismissRecorder()
    let latest = Self.retry()

    center.present(Self.failure()) { first.record(cleared: true) }
    center.present(latest) { second.record(cleared: true) }

    #expect(center.current == latest)

    center.confirm()

    #expect(first.count == 0)
    #expect(second.count == 1)
  }

  @Test func confirmResolvesTrueOnConfirm() async {
    let center = TDialogCenter()
    async let answer = center.confirm(Self.remove())
    await waitUntilPresented(center)
    #expect(center.current?.cancelText == Self.cancelText)
    center.confirm()
    #expect(await answer == true)
    #expect(center.current == nil)
  }

  @Test func confirmResolvesFalseOnDismiss() async {
    let center = TDialogCenter()
    async let answer = center.confirm(Self.remove())
    await waitUntilPresented(center)
    center.dismiss()
    #expect(await answer == false)
    #expect(center.current == nil)
  }

  @Test func announcementListsActionsOnlyForConfirm() {
    #expect(
      TDialogOverlay.announcement(for: Self.failure())
        == "\(Self.failureTitle). \(Self.failureMessage)")
    #expect(
      TDialogOverlay.announcement(for: Self.remove())
        == "\(Self.removeTitle). \(Self.removeMessage) \(Self.cancelText), \(Self.removeText)")
  }

  @Test func alertItemHasNoCancel() {
    #expect(Self.failure().cancelText == nil)
    #expect(Self.failure().confirmIsDestructive == false)
  }
}

@MainActor
private func waitUntilPresented(_ center: TDialogCenter) async {
  while center.current == nil { await Task.yield() }
}

@MainActor
private final class DismissRecorder {
  private(set) var count = 0
  private(set) var clearedWhenCalled: Bool?

  func record(cleared: Bool) {
    count += 1
    clearedWhenCalled = cleared
  }
}
