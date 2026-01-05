package app.tauri.connectionpersistence

import android.app.Activity
import android.content.Intent
import androidx.core.content.ContextCompat
import app.tauri.Logger
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class ActiveArgs {
  var active: Boolean = false
}

@TauriPlugin
class ConnectionPersistencePlugin(private val activity: Activity) : Plugin(activity) {
  private fun prefs() =
    activity.getSharedPreferences(PREFS_NAME, Activity.MODE_PRIVATE)

  private fun captureDisconnectIntent(intent: Intent?) {
    if (intent?.action != ACTION_DISCONNECT_ALL) return
    prefs().edit().putBoolean(KEY_DISCONNECT_REQUESTED, true).apply()
    // Clear the action so we don't re-run disconnect on future resumes.
    intent.action = null
    activity.intent = intent
  }

  override fun load(webView: android.webkit.WebView) {
    super.load(webView)
    captureDisconnectIntent(activity.intent)
  }

  override fun onNewIntent(intent: Intent) {
    captureDisconnectIntent(intent)
  }

  override fun onPause() {
    // Activity is leaving foreground; if there are active sessions, start the FGS to keep sockets alive.
    val active = prefs().getBoolean(KEY_ACTIVE, false)
    if (!active) return
    try {
      val intent = Intent(activity, ConnectionPersistenceService::class.java)
      ContextCompat.startForegroundService(activity, intent)
    } catch (e: Exception) {
      Logger.error("Failed to start background persistence service: ${e.message}")
    }
  }

  override fun onResume() {
    captureDisconnectIntent(activity.intent)
    // Always stop the FGS when returning to foreground.
    try {
      val intent = Intent(activity, ConnectionPersistenceService::class.java)
      activity.stopService(intent)
    } catch (_: Exception) {
      // ignore
    }
  }

  @Command
  fun start(invoke: Invoke) {
    try {
      val intent = Intent(activity, ConnectionPersistenceService::class.java)
      ContextCompat.startForegroundService(activity, intent)
      invoke.resolve(JSObject())
    } catch (e: Exception) {
      invoke.reject(e.message ?: "Failed to start background persistence")
    }
  }

  @Command
  fun stop(invoke: Invoke) {
    try {
      val intent = Intent(activity, ConnectionPersistenceService::class.java)
      activity.stopService(intent)
      invoke.resolve(JSObject())
    } catch (e: Exception) {
      invoke.reject(e.message ?: "Failed to stop background persistence")
    }
  }

  @Command
  fun setActive(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(ActiveArgs::class.java)
      prefs().edit().putBoolean(KEY_ACTIVE, args.active).apply()
      invoke.resolve(JSObject())
    } catch (e: Exception) {
      invoke.reject(e.message ?: "Failed to update background persistence state")
    }
  }

  @Command
  fun consumeDisconnectRequest(invoke: Invoke) {
    try {
      val p = prefs()
      val requested = p.getBoolean(KEY_DISCONNECT_REQUESTED, false)
      if (requested) {
        p.edit().remove(KEY_DISCONNECT_REQUESTED).apply()
      }
      val out = JSObject()
      out.put("requested", requested)
      invoke.resolve(out)
    } catch (e: Exception) {
      invoke.reject(e.message ?: "Failed to read disconnect request")
    }
  }

  companion object {
    private const val PREFS_NAME = "driftcode_connection_persistence"
    private const val KEY_DISCONNECT_REQUESTED = "disconnect_requested"
    private const val KEY_ACTIVE = "active"
    private const val ACTION_DISCONNECT_ALL = "app.tauri.connectionpersistence.action.DISCONNECT_ALL"
  }
}
