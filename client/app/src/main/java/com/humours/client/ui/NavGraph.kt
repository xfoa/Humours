package com.humours.client.ui

import androidx.compose.runtime.Composable
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController

object Routes {
    const val CONNECTION = "connection"
    const val GRID = "grid"
}

@Composable
fun NavGraph() {
    val navController = rememberNavController()
    NavHost(navController, startDestination = Routes.CONNECTION) {
        composable(Routes.CONNECTION) {
            ConnectionScreen(onConnected = {
                navController.navigate(Routes.GRID) {
                    popUpTo(Routes.CONNECTION) { inclusive = false }
                }
            })
        }
        composable(Routes.GRID) {
            GridScreen(onDisconnect = {
                navController.popBackStack(Routes.CONNECTION, inclusive = false)
            })
        }
    }
}