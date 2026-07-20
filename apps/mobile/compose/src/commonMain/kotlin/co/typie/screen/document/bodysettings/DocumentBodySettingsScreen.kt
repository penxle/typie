package co.typie.screen.document.bodysettings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.editor.DefaultRootPaginatedLayout
import co.typie.editor.DocumentEditingSession
import co.typie.editor.DocumentProtectedReloadResult
import co.typie.editor.DocumentReloadFailureDecision
import co.typie.editor.Editor
import co.typie.editor.EditorScope
import co.typie.editor.currentEditorThemeVariant
import co.typie.editor.enqueueRootLayoutMode
import co.typie.editor.enqueueRootModifier
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.ffi.Viewport
import co.typie.editor.preview.EditorPreview
import co.typie.editor.runProtectedDocumentReload
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.sync.ActiveDocumentEditingSessions
import co.typie.editor.sync.ChangesetDeltaStore
import co.typie.editor.sync.RemoteChangesetPipeline
import co.typie.editor.sync.SyncEngine
import co.typie.editor.sync.asSyncEditor
import co.typie.editor.sync.concatChangesets
import co.typie.editor.sync.isPermanentSyncError
import co.typie.editor.sync.orphanSweeper
import co.typie.editor.sync.syncAppScope
import co.typie.editor.sync.ws.AttachEvent
import co.typie.editor.sync.ws.DocumentSyncBaseline
import co.typie.editor.sync.ws.SyncWs
import co.typie.editor.sync.ws.WsSyncTransport
import co.typie.editor.sync.ws.replacementSnapshotInFlight
import co.typie.ext.imePadding
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.navigation.Nav
import co.typie.navigation.RouteRemovalDecision
import co.typie.platform.PlatformModule
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.screen.editor.editor.EditorRouteLeaveInterceptor
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.dialog.error
import co.typie.ui.component.editorsettings.EditorSettingsBasicStyleSection
import co.typie.ui.component.editorsettings.EditorSettingsDetailLayoutSection
import co.typie.ui.component.editorsettings.EditorSettingsLayoutSection
import co.typie.ui.component.editorsettings.EditorSettingsSectionDivider
import co.typie.ui.component.editorsettings.EditorStyleSettings
import co.typie.ui.component.editorsettings.changedEditorModifiersFrom
import co.typie.ui.component.editorsettings.toEditorModifiers
import co.typie.ui.component.editorsettings.toEditorStyleSettings
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.time.Clock
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.cancel
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@OptIn(ExperimentalAtomicApi::class)
@Composable
fun DocumentBodySettingsScreen(entityId: String) {
  val nav = Nav.current
  val dialog = LocalDialog.current
  val model = viewModel { DocumentBodySettingsViewModel(entityId) }
  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()
  val sheet = LocalSheet.current
  val toast = LocalToast.current
  val focusManager = LocalFocusManager.current

  ProvideTopBar(
    center = { Text("본문 설정", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(
    loadable = model.query,
    background = AppTheme.colors.surfaceInset,
    contentPadding = PaddingValues.Zero,
  ) { contentPadding ->
    val data = (model.query.state as? QueryState.Success)?.data ?: return@Screen
    val document = data.entity.node.onDocument ?: return@Screen
    val settingsRuntime = remember(document.id) { EditorRuntime(uiScope = scope) }
    val previewRuntime = remember(document.id) { EditorRuntime(uiScope = scope) }
    var load by remember(document.id) { mutableStateOf<DocumentBodySettingsLoad?>(null) }
    var liveLoad by remember(document.id) { mutableStateOf<DocumentBodySettingsLoad?>(null) }
    var reloadRequest by
      remember(document.id) { mutableStateOf<DocumentBodySettingsReloadRequest?>(null) }
    var loaderFailedCode by remember(document.id) { mutableStateOf<String?>(null) }
    var routeLeaveActive by remember(document.id) { mutableStateOf(false) }
    val channel = remember(document.id) { SyncWs.channel(document.id) }
    val graph = load?.graph

    val colors = AppTheme.colors
    val layoutDirection = LocalLayoutDirection.current
    val topBarClearance = contentPadding.calculateTopPadding()
    val previewHeight = 200.dp
    val previewContainerHeight = topBarClearance + previewHeight
    val previewShape = RoundedCornerShape(bottomStart = AppShapes.xl, bottomEnd = AppShapes.xl)
    val graphKey = remember(graph) { graph?.let { it.size to it.contentHashCode() } }
    var initial by
      remember(document.id, graphKey) { mutableStateOf<DocumentBodySettingsInitialState?>(null) }
    val previewGraph = if (initial?.hasText == true) graph else null
    var bodyStyle by remember(document.id) { mutableStateOf<EditorStyleSettings?>(null) }
    val editorThemeVariant = currentEditorThemeVariant()
    var layout by remember(document.id) { mutableStateOf<LayoutMode?>(null) }
    val resolvedBodyStyle = bodyStyle ?: EditorStyleSettings()
    val resolvedLayout = layout ?: DefaultRootPaginatedLayout
    val controlsEnabled =
      initial != null &&
        settingsRuntime.session != null &&
        liveLoad === load &&
        reloadRequest == null &&
        !routeLeaveActive

    var savingToastId by remember(document.id, toast) { mutableStateOf<Long?>(null) }

    fun dismissSavingToast() {
      val id = savingToastId ?: return
      savingToastId = null
      if (toast.state?.id == id) toast.dismiss()
    }

    fun finishReloadRequest(request: DocumentBodySettingsReloadRequest) {
      if (reloadRequest !== request) return
      reloadRequest = null
      request.policyJob = null
      request.completion.complete(Unit)
    }

    fun claimReloadReplacement(request: DocumentBodySettingsReloadRequest): Boolean {
      if (
        reloadRequest !== request ||
          settingsRuntime.session !== request.session ||
          load !== request.load ||
          liveLoad !== request.load
      ) {
        return false
      }

      val needsSnapshot = !request.snapshotInFlight
      settingsRuntime.clear(request.session)
      liveLoad = null
      load = null
      finishReloadRequest(request)
      if (needsSnapshot) SyncWs.retryDocument(document.id)
      return true
    }

    fun launchReloadPolicy(
      request: DocumentBodySettingsReloadRequest
    ): CompletableDeferred<Boolean> {
      val acquired = CompletableDeferred<Boolean>()
      if (
        routeLeaveActive ||
          reloadRequest !== request ||
          settingsRuntime.session !== request.session ||
          load !== request.load ||
          liveLoad !== request.load ||
          request.policyJob != null
      ) {
        acquired.complete(false)
        return acquired
      }

      val job =
        scope.launch(start = CoroutineStart.UNDISPATCHED) {
          try {
            when (
              runProtectedDocumentReload(
                session = request.session,
                finalizeInput = {
                  focusManager.clearFocus()
                  settingsRuntime.blur()
                  request.session.editor.sync {
                    enqueue(Message.System(SystemEvent.SetFocused(false)))
                  }
                  settingsRuntime.deactivateScene()
                },
                onStopAcquired = { acquired.complete(true) },
                showDelayedFeedback = {
                  toast.show(ToastType.Loading, "저장 중…")
                  savingToastId = toast.state?.id
                },
                hideDelayedFeedback = { dismissSavingToast() },
                resolveFailure = {
                  val result =
                    dialog.confirm(
                      title = "최신 버전을 불러올 수 없어요",
                      message = "최근 변경사항을 안전하게 저장하지 못했어요.",
                      confirmText = "변경사항 버리고 불러오기",
                      cancelText = "다시 시도",
                      confirmIsDestructive = true,
                    )
                  if (result is DialogResult.Resolved) {
                    DocumentReloadFailureDecision.Discard
                  } else {
                    DocumentReloadFailureDecision.Retry
                  }
                },
                replaceIfCurrent = { claimReloadReplacement(request) },
              )
            ) {
              DocumentProtectedReloadResult.Replaced -> Unit
              DocumentProtectedReloadResult.NotCurrent,
              DocumentProtectedReloadResult.SessionStopped -> finishReloadRequest(request)
            }
          } catch (e: CancellationException) {
            throw e
          } catch (e: Throwable) {
            if (reloadRequest === request && settingsRuntime.session === request.session) {
              finishReloadRequest(request)
              settingsRuntime.reportError(request.session, e)
            }
          }
        }
      request.policyJob = job
      job.invokeOnCompletion {
        acquired.complete(false)
        if (request.policyJob === job) request.policyJob = null
      }
      return acquired
    }

    suspend fun requestReload(
      session: DocumentEditingSession,
      activeLoad: DocumentBodySettingsLoad,
      snapshotInFlight: Boolean,
    ) {
      val current = reloadRequest
      val request =
        if (current?.session === session && current.load === activeLoad) {
          current.also { it.snapshotInFlight = it.snapshotInFlight || snapshotInFlight }
        } else {
          current?.policyJob?.cancel()
          current?.let { finishReloadRequest(it) }
          if (
            settingsRuntime.session !== session || load !== activeLoad || liveLoad !== activeLoad
          ) {
            return
          }
          DocumentBodySettingsReloadRequest(
              session = session,
              load = activeLoad,
              snapshotInFlight = snapshotInFlight,
            )
            .also { reloadRequest = it }
        }
      if (!routeLeaveActive && request.policyJob == null) {
        launchReloadPolicy(request)
      }
      request.completion.await()
    }

    LaunchedEffect(document.id, channel) {
      val snapshotChunks = mutableListOf<ByteArray>()
      channel.freshSubscribe().collect { event ->
        val replacementSnapshotInFlight = event.replacementSnapshotInFlight()
        if (replacementSnapshotInFlight != null) {
          if (replacementSnapshotInFlight) snapshotChunks.clear()

          val session = settingsRuntime.session
          val activeLoad = load
          if (session != null && activeLoad != null && liveLoad === activeLoad) {
            requestReload(session, activeLoad, replacementSnapshotInFlight)
          } else if (event is AttachEvent.PermanentErrorEvent) {
            load = null
            loaderFailedCode = event.code
          } else if (session == null) {
            load = null
          }
          return@collect
        }

        when (event) {
          is AttachEvent.SnapshotChunkEvent -> snapshotChunks += event.bytes
          is AttachEvent.SnapshotEndEvent -> {
            val candidate =
              DocumentBodySettingsLoad(
                graph = snapshotChunks.concatChangesets(),
                baseline =
                  DocumentSyncBaseline(
                    seq = event.seq,
                    heads = event.heads.copyOf(),
                    durableHeads = event.durableHeads.copyOf(),
                  ),
              )
            snapshotChunks.clear()
            if (settingsRuntime.session == null) {
              load = candidate
            }
          }
          is AttachEvent.ChangesetsEvent -> load?.queue(event)
          AttachEvent.SnapshotRestart,
          AttachEvent.ReloadEvent,
          is AttachEvent.PermanentErrorEvent -> error("Replacement event was not classified")
        }
      }
    }

    LaunchedEffect(loaderFailedCode) {
      loaderFailedCode ?: return@LaunchedEffect
      dialog.error(nav) {
        loaderFailedCode = null
        load = null
        SyncWs.retryDocument(document.id)
      }
    }

    LaunchedEffect(settingsRuntime.error) {
      settingsRuntime.error ?: return@LaunchedEffect
      dialog.error(nav) {
        settingsRuntime.clearError()
        load = null
        SyncWs.retryDocument(document.id)
      }
    }

    LaunchedEffect(graphKey) {
      val currentGraph = graph ?: return@LaunchedEffect
      val nextInitial =
        withContext(Dispatchers.Default) {
          DocumentBodySettingsInitialState(
            hasText =
              runCatching {
                  PlatformModule.editorHost.extractTextFromGraph(currentGraph).isNotBlank()
                }
                .getOrDefault(true),
            style =
              runCatching { PlatformModule.editorHost.rootModifiersFromGraph(currentGraph) }
                .getOrDefault(emptyList())
                .toEditorStyleSettings(),
            layout =
              runCatching { PlatformModule.editorHost.rootAttrsFromGraph(currentGraph).layoutMode }
                .getOrDefault(DefaultRootPaginatedLayout),
          )
        }
      initial = nextInitial
      if (bodyStyle == null) bodyStyle = nextInitial.style
      if (layout == null) layout = nextInitial.layout
    }

    LaunchedEffect(load, document.id, channel) {
      val readyLoad = load ?: return@LaunchedEffect
      if (!settingsRuntime.canCreateEditor) return@LaunchedEffect
      val pending = ChangesetDeltaStore.load(document.id).map { it.changeset }
      if (load !== readyLoad) return@LaunchedEffect
      val bootstrapFailure = AtomicReference<Throwable?>(null)
      var readyEditor: Editor? = null
      var session: DocumentEditingSession? = null
      var attached = false
      var registered = false
      val engineScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

      try {
        val createdEditor =
          Editor.createWithPending(
            graph = readyLoad.graph,
            pending = pending,
            viewport = Viewport(width = 1f, height = 1f, scaleFactor = 1.0),
            scope = scope,
            themeVariant = editorThemeVariant,
            onError = { activeEditor, error ->
              if (settingsRuntime.editor === activeEditor) {
                settingsRuntime.reportError(activeEditor, error)
              } else {
                bootstrapFailure.compareAndSet(expectedValue = null, newValue = error)
              }
            },
          )
        readyEditor = createdEditor
        bootstrapFailure.load()?.let { throw it }

        lateinit var createdSession: DocumentEditingSession
        readyLoad.activate(
          apply = { event ->
            for (bundle in event.bundles) {
              if (bundle.isNotEmpty()) createdEditor.receiveRemoteChangeset(bundle)
              bootstrapFailure.load()?.let { throw it }
            }
          },
          startLive = { baseline ->
            suspend fun awaitStreamReloadDecision() {
              requestReload(createdSession, readyLoad, snapshotInFlight = true)
            }
            suspend fun awaitPullReloadDecision() {
              requestReload(createdSession, readyLoad, snapshotInFlight = false)
            }

            val transport =
              WsSyncTransport(
                channel = channel,
                connection = SyncWs.connection,
                documentId = document.id,
                onReload = { awaitStreamReloadDecision() },
                scope = engineScope,
              )
            val engine =
              SyncEngine(
                editor = createdEditor.asSyncEditor(),
                documentId = document.id,
                initialServerHeads = baseline.heads,
                initialDurableHeads = baseline.durableHeads,
                store = ChangesetDeltaStore,
                pushFn = { transport.push(it) },
                scope = engineScope,
                isPermanent = ::isPermanentSyncError,
                now = { Clock.System.now().toEpochMilliseconds() },
              )
            val pipeline =
              RemoteChangesetPipeline(
                editor = createdEditor.asSyncEditor(),
                headsSink = engine,
                transport = transport,
                initialSeq = baseline.seq,
                scope = engineScope,
                onNeedsReload = { awaitPullReloadDecision() },
              )
            createdSession =
              DocumentEditingSession(
                documentId = document.id,
                editor = createdEditor,
                engine = engine,
                pipeline = pipeline,
                scope = engineScope,
              )
            session = createdSession
            settingsRuntime.attach(createdSession)
            check(settingsRuntime.session === createdSession)
            attached = true
            createdSession.start()
            ActiveDocumentEditingSessions.register(createdSession)
            registered = true
            liveLoad = readyLoad
          },
        )

        awaitCancellation()
      } catch (e: CancellationException) {
        throw e
      } catch (e: Throwable) {
        if (load === readyLoad) settingsRuntime.reportError(e)
      } finally {
        if (liveLoad === readyLoad) liveLoad = null
        val closingSession = session
        val closingRequest = reloadRequest?.takeIf {
          it.session === closingSession && it.load === readyLoad
        }
        closingRequest?.policyJob?.cancel()
        closingRequest?.let { finishReloadRequest(it) }
        if (closingSession != null) {
          settingsRuntime.clear(closingSession)
          closingSession.stop()
          if (registered) ActiveDocumentEditingSessions.unregister(closingSession)
        } else {
          readyEditor?.dispose()
        }
        engineScope.cancel()
        if (attached) syncAppScope.launch { orphanSweeper.sweep() }
      }
    }

    val settingsEditor = settingsRuntime.editor
    val authoritativeRootAttrs = settingsEditor?.rootAttrs
    val authoritativeRootModifiers = settingsEditor?.rootModifiers
    LaunchedEffect(settingsEditor, authoritativeRootAttrs, authoritativeRootModifiers) {
      val rootAttrs = authoritativeRootAttrs ?: return@LaunchedEffect
      val rootModifiers = authoritativeRootModifiers ?: return@LaunchedEffect
      layout = rootAttrs.layoutMode
      bodyStyle = rootModifiers.toEditorStyleSettings()
    }

    val leaveInterceptor =
      remember(settingsEditor, settingsRuntime.session, dialog, toast) {
        val activeEditor = settingsEditor ?: return@remember null
        val activeSession =
          settingsRuntime.session?.takeIf { it.editor === activeEditor } ?: return@remember null
        EditorRouteLeaveInterceptor(
          finalizeInput = {
            focusManager.clearFocus()
            settingsRuntime.blur()
            activeEditor.sync { enqueue(Message.System(SystemEvent.SetFocused(false))) }
            settingsRuntime.deactivateScene()
          },
          restoreInput = {},
          beginStop = activeSession::beginStop,
          onPreparationStarted = {
            routeLeaveActive = true
            try {
              val request = reloadRequest?.takeIf {
                it.session === activeSession && it.load === load
              }
              request?.policyJob?.cancelAndJoin()
            } catch (throwable: Throwable) {
              routeLeaveActive = false
              throw throwable
            }
          },
          resumeReloadBeforeRollback = {
            routeLeaveActive = false
            val request = reloadRequest?.takeIf {
              it.session === activeSession &&
                settingsRuntime.session === activeSession &&
                it.load === load &&
                liveLoad === it.load
            }
            if (request == null) {
              false
            } else {
              val stopAcquired = launchReloadPolicy(request).await()
              val reloadOwnsStop =
                stopAcquired &&
                  (request.policyJob?.isActive == true || settingsRuntime.session !== activeSession)
              if (!reloadOwnsStop) finishReloadRequest(request)
              reloadOwnsStop
            }
          },
          showDelayedFeedback = {
            toast.show(ToastType.Loading, "저장 중…")
            savingToastId = toast.state?.id
          },
          hideDelayedFeedback = { dismissSavingToast() },
          resolveDecision = {
            val result =
              dialog.confirm(
                title = "저장을 완료하지 못했어요",
                message = "지금 닫으면 최근 변경사항을 잃을 수 있어요.",
                confirmText = "저장하지 않고 닫기",
                cancelText = "계속 편집",
                confirmIsDestructive = true,
              )
            if (result is DialogResult.Resolved) {
              RouteRemovalDecision.ProceedWithRemoval
            } else {
              RouteRemovalDecision.CancelRemoval
            }
          },
        )
      }
    DisposableEffect(nav, entityId, leaveInterceptor) {
      val unregister = leaveInterceptor?.let {
        nav.routeRemovals.register(Route.DocumentBodySettings(entityId), it)
      }
      onDispose {
        unregister?.invoke()
        dismissSavingToast()
      }
    }

    fun save(block: EditorScope.() -> Unit) {
      if (!controlsEnabled) return
      val activeSession = settingsRuntime.session ?: return
      activeSession.submit { activeEditor, context ->
        activeEditor.scope.launch(context) {
          model
            .applyAndRelayBodySettings(editor = activeEditor, block = block)
            .withDefaultExceptionHandler(toast)
        }
      }
    }

    fun saveLayout(newLayout: LayoutMode) {
      if (!controlsEnabled) return
      layout = newLayout
      save {
        enqueueRootLayoutMode(newLayout)
        enqueue(Message.System(SystemEvent.SetFocused(false)))
      }
    }

    fun saveStyle(newStyle: EditorStyleSettings) {
      if (!controlsEnabled) return
      val modifiers = newStyle.changedEditorModifiersFrom(resolvedBodyStyle)
      if (modifiers.isEmpty()) return
      bodyStyle = newStyle
      save {
        modifiers.forEach { modifier -> enqueueRootModifier(modifier) }
        enqueue(Message.System(SystemEvent.SetFocused(false)))
      }
    }

    Box(
      modifier =
        Modifier.fillMaxSize()
          .imePadding()
          .padding(
            start = contentPadding.calculateStartPadding(layoutDirection),
            end = contentPadding.calculateEndPadding(layoutDirection),
          )
    ) {
      Skeleton(enabled = !controlsEnabled, modifier = Modifier.matchParentSize()) {
        Column(
          modifier =
            Modifier.fillMaxSize()
              .verticalScroll(scrollState)
              .background(colors.surfaceDefault)
              .padding(
                top = previewContainerHeight + 12.dp,
                bottom = contentPadding.calculateBottomPadding(),
              )
              .padding(AppTheme.spacings.scrollBottomPadding)
        ) {
          EditorSettingsBasicStyleSection(
            style = resolvedBodyStyle,
            fontFamilies = model.fontFamilies,
            sheet = sheet,
            onStyleChange = { style -> saveStyle(style) },
          )

          EditorSettingsSectionDivider()

          EditorSettingsLayoutSection(
            layout = resolvedLayout,
            sheet = sheet,
            onLayoutChange = { layout -> saveLayout(layout) },
          )

          EditorSettingsSectionDivider()

          EditorSettingsDetailLayoutSection(
            style = resolvedBodyStyle,
            onStyleChange = { style -> saveStyle(style) },
          )
        }
      }

      Box(modifier = Modifier.fillMaxWidth()) {
        if (initial != null) {
          EditorPreview(
            layoutMode = resolvedLayout,
            runtime = previewRuntime,
            modifier = Modifier.fillMaxWidth().height(previewContainerHeight).zIndex(1f),
            shape = previewShape,
            contentTopPadding = topBarClearance,
            graph = previewGraph,
            modifiers = resolvedBodyStyle.toEditorModifiers(),
          )
        } else {
          Skeleton.Bone(
            modifier = Modifier.fillMaxWidth().height(previewContainerHeight).zIndex(1f),
            shape = previewShape,
          )
        }

        Box(
          modifier =
            Modifier.fillMaxWidth()
              .height(16.dp + AppShapes.xl / 2)
              .offset(y = previewContainerHeight - AppShapes.xl / 2)
              .background(
                Brush.verticalGradient(
                  colors = listOf(colors.surfaceInset, colors.surfaceInset.copy(alpha = 0f))
                )
              )
        )
      }
    }
  }
}

private data class DocumentBodySettingsInitialState(
  val hasText: Boolean,
  val style: EditorStyleSettings,
  val layout: LayoutMode,
)

internal class DocumentBodySettingsLoad(val graph: ByteArray, baseline: DocumentSyncBaseline) {
  private val pending = mutableListOf<AttachEvent.ChangesetsEvent>()
  private var currentBaseline = baseline
  private var live = false

  fun queue(event: AttachEvent.ChangesetsEvent): Boolean {
    if (live) return false
    pending += event
    return true
  }

  suspend fun activate(
    apply: suspend (AttachEvent.ChangesetsEvent) -> Unit,
    startLive: (DocumentSyncBaseline) -> Unit,
  ) {
    check(!live)
    while (pending.isNotEmpty()) {
      val batch = pending.toList()
      pending.clear()
      for (event in batch) {
        apply(event)
        currentBaseline =
          currentBaseline.copy(
            seq = event.seq.ifEmpty { currentBaseline.seq },
            heads = event.heads.copyOf(),
            durableHeads = event.durableHeads.copyOf(),
          )
      }
    }
    startLive(currentBaseline)
    check(pending.isEmpty()) { "Changeset queued during non-suspending live cutover" }
    live = true
  }
}

private class DocumentBodySettingsReloadRequest(
  val session: DocumentEditingSession,
  val load: DocumentBodySettingsLoad,
  snapshotInFlight: Boolean,
) {
  val completion = CompletableDeferred<Unit>()
  var policyJob: Job? = null
  var snapshotInFlight = snapshotInFlight
}
