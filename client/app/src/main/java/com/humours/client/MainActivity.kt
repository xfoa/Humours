package com.humours.client

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import com.humours.client.ui.NavGraph

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
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