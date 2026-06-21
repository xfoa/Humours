package com.humours.client

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import com.humours.client.ui.theme.HumoursTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            HumoursTheme {
                Surface(color = MaterialTheme.colorScheme.background) {
                    // TODO: Set up NavHost with GridScreen and SettingsScreen
                }
            }
        }
    }
}
