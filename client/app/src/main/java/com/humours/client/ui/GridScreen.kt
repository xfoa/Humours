package com.humours.client.ui

import android.view.SurfaceView
import android.view.ViewGroup
import android.widget.FrameLayout
import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ViewQuilt
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.DragHandle
import androidx.compose.material.icons.filled.Fullscreen
import androidx.compose.material.icons.filled.FullscreenExit
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.PowerSettingsNew
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.humours.client.HumoursApplication
import com.humours.client.builtin.CubeTetrahedronPlugin
import com.humours.client.data.local.StoredWidget
import com.humours.client.plugin.PluginInstance
import com.humours.client.plugin.WidgetPlugin
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlin.math.roundToInt

private data class WidgetPlacement(
    val id: String,
    val plugin: String,
    var x: Int,
    var y: Int,
    var width: Int,
    var height: Int,
    var maximised: Boolean = false,
)

private fun WidgetPlacement.toStored(): StoredWidget =
    StoredWidget(id, plugin, x, y, width, height, maximised)

private fun metricsFor(plugin: String): List<String> = CubeTetrahedronPlugin.CUBE_METRICS

private fun realScreenSizePx(context: android.content.Context): Pair<Int, Int> {
    val wm = context.getSystemService(android.content.Context.WINDOW_SERVICE) as android.view.WindowManager
    return if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
        val b = wm.currentWindowMetrics.bounds
        b.width() to b.height()
    } else {
        val point = android.graphics.Point()
        @Suppress("DEPRECATION")
        wm.defaultDisplay.getRealSize(point)
        point.x to point.y
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GridScreen(
    onDisconnect: () -> Unit,
    onNavigate: (String) -> Unit,
) {
    val context = LocalContext.current
    val app = remember { context.applicationContext as HumoursApplication }
    val scope = rememberCoroutineScope()
    val density = LocalDensity.current
    val defaultSizePx = remember { with(density) { 180.dp.toPx().roundToInt() } }
    var canvasWidthPx by remember { mutableStateOf(0) }
    var canvasHeightPx by remember { mutableStateOf(0) }
    val fallback = remember(context) { realScreenSizePx(context) }
    val screenWidthPx = if (canvasWidthPx > 0) canvasWidthPx else fallback.first
    val screenHeightPx = if (canvasHeightPx > 0) canvasHeightPx else fallback.second
    val gridCellXPx = remember(screenWidthPx) {
        fitCell(screenWidthPx, 12)
    }
    val gridCellYPx = remember(screenHeightPx, gridCellXPx) {
        val rows = (screenHeightPx.toFloat() / gridCellXPx.coerceAtLeast(1))
            .roundToInt().coerceAtLeast(1)
        fitCell(screenHeightPx, rows)
    }

    var overlayVisible by remember { mutableStateOf(false) }
    var pluginSheetVisible by remember { mutableStateOf(false) }
    var addSheetVisible by remember { mutableStateOf(false) }
    var layoutMode by remember { mutableStateOf(false) }
    var cubesVisible by remember { mutableStateOf(true) }
    var loaded by remember { mutableStateOf(false) }
    var previewRect by remember { mutableStateOf<IntRect?>(null) }

    var widgets by remember { mutableStateOf<List<WidgetPlacement>>(emptyList()) }

    val defaultPlugin = app.builtInPluginClassName

    LaunchedEffect(Unit) {
        val stored = app.layoutStore.layout.first()
        if (stored.isEmpty()) {
            widgets = listOf(
                WidgetPlacement(
                    id = "widget-cube",
                    plugin = defaultPlugin,
                    x = 0, y = 0,
                    width = defaultSizePx,
                    height = defaultSizePx,
                )
            )
        } else {
            widgets = stored.map {
                WidgetPlacement(it.id, it.plugin, it.x, it.y, it.width, it.height, it.maximised)
            }
        }
        loaded = true
    }

    LaunchedEffect(widgets, layoutMode) {
        if (!loaded) return@LaunchedEffect
        app.layoutStore.save(widgets.map { it.toStored() })
    }

    fun doDisconnect() {
        cubesVisible = false
        app.pluginInstanceManager.shutdown()
        app.connectionManager.disconnect()
        onDisconnect()
    }

    fun removeWidget(id: String) {
        widgets = widgets.filter { it.id != id }
        app.pluginInstanceManager.destroy(id)
    }

    fun addWidget(pluginClass: String) {
        val base = pluginClass.substringAfterLast('.').lowercase()
        var id = "$base-${widgets.size + 1}"
        var i = widgets.size + 1
        while (widgets.any { it.id == id }) { i++; id = "$base-$i" }
        widgets = widgets + WidgetPlacement(id, pluginClass, 0, 0, defaultSizePx, defaultSizePx)
        registerPlugin(app, id, pluginClass)
    }

    fun toggleLayout() {
        layoutMode = !layoutMode
        overlayVisible = false
    }

    BackHandler(enabled = true) {
        when {
            overlayVisible -> overlayVisible = false
            pluginSheetVisible -> pluginSheetVisible = false
            addSheetVisible -> addSheetVisible = false
            layoutMode -> layoutMode = false
            else -> Unit
        }
    }

    DisposableEffect(Unit) {
        onDispose {
            app.pluginInstanceManager.shutdown()
            app.surfaceManager.detachOverlay()
        }
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black)
            .onSizeChanged {
                canvasWidthPx = it.width
                canvasHeightPx = it.height
            }
    ) {
        if (cubesVisible) {
            widgets.forEach { placement ->
                WidgetCanvasCell(
                    placement = placement,
                    pluginClass = placement.plugin,
                    layoutMode = layoutMode,
                    gridCellX = gridCellXPx,
                    gridCellY = gridCellYPx,
                    screenWidth = screenWidthPx,
                    screenHeight = screenHeightPx,
                    onDelete = { removeWidget(placement.id) },
                    onPreview = { previewRect = it },
                )
            }
        }

        if (layoutMode) {
            GridOverlay(
                gridCellX = gridCellXPx,
                gridCellY = gridCellYPx,
                screenWidth = screenWidthPx,
                screenHeight = screenHeightPx,
            )
        }

        previewRect?.let { rect ->
            Box(
                modifier = Modifier
                    .offset { IntOffset(rect.left, rect.top) }
                    .size(
                        width = with(density) { rect.width.toDp() },
                        height = with(density) { rect.height.toDp() },
                    )
                    .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.18f))
                    .border(
                        width = 2.dp,
                        color = MaterialTheme.colorScheme.primary,
                        shape = RoundedCornerShape(12.dp),
                    )
            )
        }

        if (!overlayVisible && !layoutMode && !pluginSheetVisible && !addSheetVisible) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .clickable(
                        interactionSource = remember { MutableInteractionSource() },
                        indication = null,
                    ) { overlayVisible = true }
            )
        }

        AnimatedVisibility(
            visible = overlayVisible,
            enter = fadeIn(),
            exit = fadeOut(),
        ) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .clickable(
                        interactionSource = remember { MutableInteractionSource() },
                        indication = null,
                    ) { overlayVisible = false }
            )
        }

        AnimatedVisibility(
            visible = overlayVisible,
            enter = fadeIn(),
            exit = fadeOut(),
            modifier = Modifier.align(Alignment.Center),
        ) {
            OverlayBar(
                onDisconnect = {
                    overlayVisible = false
                    doDisconnect()
                },
                onToggleLayout = ::toggleLayout,
                onOpenPluginSettings = {
                    overlayVisible = false
                    pluginSheetVisible = true
                },
                onAbout = {
                    overlayVisible = false
                    onNavigate(Routes.ABOUT)
                },
            )
        }

        if (layoutMode) {
            LayoutModeBanner(
                modifier = Modifier.align(Alignment.TopCenter),
            )
            LayoutActionCluster(
                modifier = Modifier.align(Alignment.BottomCenter),
                onAdd = { addSheetVisible = true },
                onExit = { layoutMode = false },
            )
        }
    }

    if (pluginSheetVisible) {
        val sheetState = rememberModalBottomSheetState()
        ModalBottomSheet(
            onDismissRequest = { pluginSheetVisible = false },
            sheetState = sheetState,
        ) {
            Column(Modifier.padding(bottom = 24.dp)) {
                Text(
                    text = "Widgets",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                )
                HorizontalDivider()
                widgets.forEach { placement ->
                    WidgetSheetRow(
                        id = placement.id,
                        pluginClass = placement.plugin,
                        onDelete = { removeWidget(placement.id) },
                    )
                    HorizontalDivider()
                }
                if (widgets.isEmpty()) {
                    Text(
                        "No widgets. Enter layout mode to add one.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(16.dp),
                    )
                }
                Spacer(Modifier.height(64.dp))
            }
        }
    }

    if (addSheetVisible) {
        val sheetState = rememberModalBottomSheetState()
        val available = remember { app.pluginLoader.availablePlugins() }
        val metadata = remember(available) {
            available.mapNotNull { cls -> cls to (app.pluginLoader.metadataFor(cls)) }
        }
        ModalBottomSheet(
            onDismissRequest = { addSheetVisible = false },
            sheetState = sheetState,
        ) {
            Column(Modifier.padding(bottom = 24.dp)) {
                Text(
                    text = "Choose a plugin to add",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                )
                HorizontalDivider()
                metadata.forEach { (cls, meta) ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable {
                                addWidget(cls)
                                addSheetVisible = false
                            }
                            .padding(horizontal = 16.dp, vertical = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(Icons.Filled.Add, contentDescription = null)
                        Spacer(Modifier.width(16.dp))
                        Column(Modifier.weight(1f)) {
                            Text(meta?.name ?: cls, style = MaterialTheme.typography.bodyLarge)
                            Text(
                                cls.substringAfterLast('.'),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                    HorizontalDivider()
                }
                Spacer(Modifier.height(64.dp))
            }
        }
    }
}

private fun registerPlugin(app: HumoursApplication, id: String, pluginClass: String) {
    val metrics = metricsFor(pluginClass)
    val plugin: WidgetPlugin = app.pluginLoader.load(pluginClass) ?: return
    app.pluginInstanceManager.register(PluginInstance(id, plugin, metrics))
}

private class WidgetGeom(
    initialX: Int,
    initialY: Int,
    initialW: Int,
    initialH: Int,
) {
    val x = mutableStateOf(initialX)
    val y = mutableStateOf(initialY)
    val w = mutableStateOf(initialW)
    val h = mutableStateOf(initialH)
    val maximised = mutableStateOf(false)

    fun displayX(): Int = if (maximised.value) 0 else x.value
    fun displayY(): Int = if (maximised.value) 0 else y.value
    fun displayW(screenW: Int): Int = if (maximised.value) screenW else w.value
    fun displayH(screenH: Int): Int = if (maximised.value) screenH else h.value
}

private fun fitCell(dimensionPx: Int, count: Int): Int {
    val c = count.coerceAtLeast(1)
    return (dimensionPx / c).coerceAtLeast(1)
}

private fun snapTargets(max: Int, cell: Int): List<Int> {
    val c = cell.coerceAtLeast(1)
    val count = (max.toFloat() / c).roundToInt().coerceAtLeast(1)
    val out = mutableListOf<Int>()
    for (i in 0..count) {
        out.add((i * max / count.toFloat()).roundToInt())
    }
    if (out.first() != 0) out.add(0, 0)
    if (out.last() != max && max >= 0) out.add(max)
    return out.distinct().sorted()
}

private fun snapTo(value: Int, targets: List<Int>): Int {
    val clamped = value.coerceIn(targets.first(), targets.last())
    return targets.minByOrNull { kotlin.math.abs(it - clamped) } ?: clamped
}

@Composable
private fun WidgetCanvasCell(
    placement: WidgetPlacement,
    pluginClass: String,
    layoutMode: Boolean,
    gridCellX: Int,
    gridCellY: Int,
    screenWidth: Int,
    screenHeight: Int,
    onDelete: () -> Unit,
    onPreview: (IntRect?) -> Unit,
) {
    val context = LocalContext.current
    val app = remember { context.applicationContext as HumoursApplication }
    val density = LocalDensity.current
    val minSize = minOf(gridCellX, gridCellY).coerceAtLeast(1)
    val geom = remember(placement.id) {
        WidgetGeom(placement.x, placement.y, placement.width, placement.height).apply {
            maximised.value = placement.maximised
        }
    }
    val xTargets = remember(screenWidth, gridCellX) { snapTargets(screenWidth, gridCellX) }
    val yTargets = remember(screenHeight, gridCellY) { snapTargets(screenHeight, gridCellY) }

    DisposableEffect(placement.id) {
        onDispose {
            app.pluginInstanceManager.destroy(placement.id)
        }
    }

    var coords by remember { mutableStateOf<LayoutCoordinates?>(null) }
    Box(
        modifier = Modifier
            .onGloballyPositioned { coords = it }
            .offset { IntOffset(geom.displayX(), geom.displayY()) }
            .size(
                width = with(density) { geom.displayW(screenWidth).toDp() },
                height = with(density) { geom.displayH(screenHeight).toDp() },
            )
            .then(
                if (layoutMode) {
                    Modifier
                        .border(
                            width = 2.dp,
                            color = MaterialTheme.colorScheme.primary,
                            shape = RoundedCornerShape(12.dp),
                        )
                        .then(
                            if (!geom.maximised.value) {
                                Modifier.pointerInput(layoutMode) {
                                    var dragTotal = Offset.Zero
                                    detectDragGestures(
                                        onDragStart = {
                                            dragTotal = Offset.Zero
                                        },
                                        onDragEnd = {
                                            val maxXw = (screenWidth - geom.w.value).coerceAtLeast(0)
                                            val maxYh = (screenHeight - geom.h.value).coerceAtLeast(0)
                                            val rawX = (geom.x.value + dragTotal.x.roundToInt()).coerceIn(0, maxXw)
                                            val rawY = (geom.y.value + dragTotal.y.roundToInt()).coerceIn(0, maxYh)
                                            val sx = snapTo(rawX, xTargets)
                                            val sy = snapTo(rawY, yTargets)
                                            geom.x.value = sx
                                            geom.y.value = sy
                                            placement.x = sx
                                            placement.y = sy
                                            onPreview(null)
                                        },
                                        onDragCancel = { onPreview(null) },
                                    ) { _, drag ->
                                        dragTotal += drag
                                        val maxXw = (screenWidth - geom.w.value).coerceAtLeast(0)
                                        val maxYh = (screenHeight - geom.h.value).coerceAtLeast(0)
                                        val rawX = (geom.x.value + dragTotal.x.roundToInt()).coerceIn(0, maxXw)
                                        val rawY = (geom.y.value + dragTotal.y.roundToInt()).coerceIn(0, maxYh)
                                        val sx = snapTo(rawX, xTargets)
                                        val sy = snapTo(rawY, yTargets)
                                        onPreview(
                                            IntRect(
                                                sx,
                                                sy,
                                                sx + geom.w.value,
                                                sy + geom.h.value,
                                            )
                                        )
                                    }
                                }
                            } else Modifier
                        )
                } else Modifier
            )
            .clip(RoundedCornerShape(12.dp)),
    ) {
        AndroidView(
            factory = { ctx ->
                FrameLayout(ctx).apply {
                    val sv = SurfaceView(ctx)
                    sv.holder.setFormat(android.graphics.PixelFormat.TRANSLUCENT)
                    sv.visibility = android.view.View.INVISIBLE
                    sv.isClickable = false
                    sv.isFocusable = false
                    sv.isEnabled = false
                    addView(
                        sv,
                        FrameLayout.LayoutParams(
                            ViewGroup.LayoutParams.MATCH_PARENT,
                            ViewGroup.LayoutParams.MATCH_PARENT,
                        )
                    )
                    setOnTouchListener { _, _ -> false }
                    isClickable = false
                    isFocusable = false
                    val metrics = metricsFor(pluginClass)
                    val plugin = app.pluginLoader.load(pluginClass)
                    if (plugin != null) {
                        app.pluginInstanceManager.register(
                            PluginInstance(placement.id, plugin, metrics)
                        )
                        app.pluginInstanceManager.startRendering(placement.id, sv) {}
                    }
                    postDelayed({ sv.visibility = android.view.View.VISIBLE }, 300)
                    this
                }
            },
            modifier = Modifier.fillMaxSize(),
        )

        if (layoutMode) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(Color.Black.copy(alpha = 0.25f))
            )
            Surface(
                shape = CircleShape,
                color = MaterialTheme.colorScheme.errorContainer,
                modifier = Modifier
                    .align(Alignment.TopStart)
                    .padding(4.dp)
                    .size(36.dp),
                onClick = onDelete,
            ) {
                Icon(
                    Icons.Filled.Delete,
                    contentDescription = "Delete widget",
                    modifier = Modifier.padding(4.dp),
                    tint = MaterialTheme.colorScheme.onErrorContainer,
                )
            }
            val maximised = geom.maximised.value
            Surface(
                shape = CircleShape,
                color = MaterialTheme.colorScheme.primaryContainer,
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .padding(4.dp)
                    .size(36.dp),
                onClick = {
                    geom.maximised.value = !maximised
                    placement.maximised = !maximised
                },
            ) {
                Icon(
                    if (maximised) Icons.Filled.FullscreenExit else Icons.Filled.Fullscreen,
                    contentDescription = if (maximised) "Restore widget" else "Maximise widget",
                    modifier = Modifier.padding(4.dp),
                    tint = MaterialTheme.colorScheme.onPrimaryContainer,
                )
            }
            var pendingW = geom.displayW(screenWidth)
            var pendingH = geom.displayH(screenHeight)
            var gestureStartW = 0
            var gestureStartH = 0
            ResizeHandle(
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .padding(4.dp),
                onResizeStart = {
                    gestureStartW = geom.displayW(screenWidth)
                    gestureStartH = geom.displayH(screenHeight)
                },
                onResize = { totalDx, totalDy ->
                    val originX = geom.displayX()
                    val originY = geom.displayY()
                    val maxW = (screenWidth - originX).coerceAtLeast(minSize)
                    val maxH = (screenHeight - originY).coerceAtLeast(minSize)
                    val rawW = (gestureStartW + totalDx).coerceIn(minSize, maxW)
                    val rawH = (gestureStartH + totalDy).coerceIn(minSize, maxH)
                    val sizeXTargets = snapTargets(maxW, gridCellX)
                    val sizeYTargets = snapTargets(maxH, gridCellY)
                    pendingW = snapTo(rawW, sizeXTargets)
                    pendingH = snapTo(rawH, sizeYTargets)
                    onPreview(
                        IntRect(
                            originX,
                            originY,
                            originX + pendingW,
                            originY + pendingH,
                        )
                    )
                },
                onResizeEnd = {
                    geom.x.value = geom.displayX()
                    geom.y.value = geom.displayY()
                    geom.w.value = pendingW
                    geom.h.value = pendingH
                    placement.x = geom.x.value
                    placement.y = geom.y.value
                    placement.width = pendingW
                    placement.height = pendingH
                    placement.maximised = false
                    geom.maximised.value = false
                    onPreview(null)
                },
            )
        }
    }
}

@Composable
private fun ResizeHandle(
    modifier: Modifier = Modifier,
    onResizeStart: () -> Unit = {},
    onResize: (Int, Int) -> Unit,
    onResizeEnd: () -> Unit = {},
) {
    Surface(
        shape = CircleShape,
        color = MaterialTheme.colorScheme.primaryContainer,
        modifier = modifier
            .size(28.dp)
            .pointerInput(Unit) {
                var total = Offset.Zero
                detectDragGestures(
                    onDragStart = {
                        total = Offset.Zero
                        onResizeStart()
                    },
                    onDragEnd = onResizeEnd,
                    onDragCancel = onResizeEnd,
                ) { _, drag ->
                    total += drag
                    onResize(total.x.roundToInt(), total.y.roundToInt())
                }
            },
    ) {
        Icon(
            Icons.Filled.DragHandle,
            contentDescription = "Resize",
            modifier = Modifier.padding(4.dp),
            tint = MaterialTheme.colorScheme.onPrimaryContainer,
        )
    }
}

@Composable
private fun GridOverlay(gridCellX: Int, gridCellY: Int, screenWidth: Int, screenHeight: Int) {
    val xTargets = remember(screenWidth, gridCellX) { snapTargets(screenWidth, gridCellX) }
    val yTargets = remember(screenHeight, gridCellY) { snapTargets(screenHeight, gridCellY) }
    Canvas(modifier = Modifier.fillMaxSize()) {
        val lineColor = Color.White.copy(alpha = 0.08f)
        xTargets.forEach { x ->
            drawLine(
                color = lineColor,
                start = androidx.compose.ui.geometry.Offset(x.toFloat(), 0f),
                end = androidx.compose.ui.geometry.Offset(x.toFloat(), size.height),
                strokeWidth = 1f,
            )
        }
        yTargets.forEach { y ->
            drawLine(
                color = lineColor,
                start = androidx.compose.ui.geometry.Offset(0f, y.toFloat()),
                end = androidx.compose.ui.geometry.Offset(size.width, y.toFloat()),
                strokeWidth = 1f,
            )
        }
    }
}

@Composable
private fun LayoutModeBanner(modifier: Modifier = Modifier) {
    Surface(
        modifier = modifier
            .padding(top = 16.dp),
        shape = RoundedCornerShape(20.dp),
        tonalElevation = 6.dp,
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                Icons.AutoMirrored.Filled.ViewQuilt,
                contentDescription = null,
                modifier = Modifier.size(18.dp),
            )
            Spacer(Modifier.width(8.dp))
            Text("Layout mode", style = MaterialTheme.typography.labelLarge)
        }
    }
}

@Composable
private fun LayoutActionCluster(
    modifier: Modifier = Modifier,
    onAdd: () -> Unit,
    onExit: () -> Unit,
) {
    Surface(
        modifier = modifier
            .padding(bottom = 24.dp),
        shape = CircleShape,
        tonalElevation = 6.dp,
        shadowElevation = 6.dp,
        color = MaterialTheme.colorScheme.surfaceVariant,
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            FilledIconButton(
                onClick = onAdd,
                modifier = Modifier.size(48.dp),
            ) {
                Icon(
                    Icons.Filled.Add,
                    contentDescription = "Add widget",
                    modifier = Modifier.size(28.dp),
                )
            }
            FilledTonalIconButton(
                onClick = onExit,
                modifier = Modifier.size(48.dp),
            ) {
                Icon(
                    Icons.Filled.Check,
                    contentDescription = "Done",
                    modifier = Modifier.size(28.dp),
                )
            }
        }
    }
}

@Composable
private fun WidgetSheetRow(id: String, pluginClass: String, onDelete: () -> Unit) {
    val app = (LocalContext.current.applicationContext as HumoursApplication)
    val meta = remember(pluginClass) { app.pluginLoader.metadataFor(pluginClass) }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { }
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(Icons.Filled.Tune, contentDescription = null)
        Spacer(Modifier.width(16.dp))
        Column(Modifier.weight(1f)) {
            Text(meta?.name ?: id, style = MaterialTheme.typography.bodyLarge)
            Text(
                id,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        IconButton(onClick = onDelete) {
            Icon(Icons.Filled.Delete, contentDescription = "Remove widget")
        }
    }
}

@Composable
private fun OverlayBar(
    onDisconnect: () -> Unit,
    onToggleLayout: () -> Unit,
    onOpenPluginSettings: () -> Unit,
    onAbout: () -> Unit,
) {
    Surface(
        shape = CircleShape,
        tonalElevation = 6.dp,
        shadowElevation = 6.dp,
        color = MaterialTheme.colorScheme.surfaceVariant,
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OverlayButton(Icons.Filled.PowerSettingsNew, "Disconnect", onDisconnect)
            OverlayButton(Icons.AutoMirrored.Filled.ViewQuilt, "Layout mode", onToggleLayout)
            OverlayButton(Icons.Filled.Tune, "Plugin settings", onOpenPluginSettings)
            OverlayButton(Icons.Filled.Info, "About", onAbout)
        }
    }
}

@Composable
private fun OverlayButton(icon: ImageVector, desc: String, onClick: () -> Unit) {
    IconButton(onClick = onClick) {
        Icon(
            icon,
            contentDescription = desc,
            modifier = Modifier.size(28.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

