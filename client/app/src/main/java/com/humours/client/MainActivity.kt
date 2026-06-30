package com.humours.client

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.Composable
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.humours.client.ui.NavGraph

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        WindowCompat.setDecorFitsSystemWindows(window, false)
        WindowInsetsControllerCompat(window, window.decorView).apply {
            hide(WindowInsetsCompat.Type.systemBars())
            systemBarsBehavior =
                WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
        setContent { HumoursApp() }
    }

    override fun onDestroy() {
        super.onDestroy()
        val app = applicationContext as? HumoursApplication ?: return
        runCatching { app.pluginInstanceManager.shutdown() }
        runCatching { app.connectionManager.disconnect() }
        runCatching { app.glThread.destroyGl() }
        runCatching { app.surfaceManager.detachOverlay() }
    }
}

@Composable
private fun HumoursApp() {
    MaterialTheme(colorScheme = darkColorScheme()) {
        NavGraph()
    }
}