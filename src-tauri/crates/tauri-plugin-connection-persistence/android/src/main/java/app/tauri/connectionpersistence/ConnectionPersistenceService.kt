package app.tauri.connectionpersistence

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat

class ConnectionPersistenceService : Service() {
  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    startForeground(NOTIFICATION_ID, buildNotification())
    return START_NOT_STICKY
  }

  private fun buildNotification(): android.app.Notification {
    ensureChannel()
    val launchIntent = packageManager.getLaunchIntentForPackage(packageName)
    val contentIntent =
      if (launchIntent != null) {
        PendingIntent.getActivity(
          this,
          0,
          launchIntent,
          PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
      } else {
        null
      }

    val disconnectActivityIntent =
      Intent(launchIntent).apply {
        action = ACTION_DISCONNECT_ALL
        addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP)
      }
    val disconnectPending =
      PendingIntent.getActivity(
        this,
        1,
        disconnectActivityIntent,
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
      )

    return NotificationCompat.Builder(this, CHANNEL_ID)
      .setContentTitle("DriftCode running in BG")
      .setContentText("Keeping SSH sessions alive")
      .setSmallIcon(android.R.drawable.stat_sys_upload)
      .setOngoing(true)
      .setSilent(true)
      .setOnlyAlertOnce(true)
      .setCategory(android.app.Notification.CATEGORY_SERVICE)
      .setContentIntent(contentIntent)
      .addAction(
        NotificationCompat.Action.Builder(
          android.R.drawable.ic_menu_close_clear_cancel,
          "Disconnect",
          disconnectPending
        ).build()
      )
      .build()
  }

  private fun ensureChannel() {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
    val mgr = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
    val existing = mgr.getNotificationChannel(CHANNEL_ID)
    if (existing != null) return
    val channel = NotificationChannel(
      CHANNEL_ID,
      "Connection persistence",
      NotificationManager.IMPORTANCE_LOW
    )
    channel.description = "Keeps DriftCode running in the background to maintain SSH connections."
    channel.setShowBadge(false)
    channel.enableVibration(false)
    channel.setSound(null, null)
    channel.lockscreenVisibility = Notification.VISIBILITY_PRIVATE
    mgr.createNotificationChannel(channel)
  }

  companion object {
    private const val CHANNEL_ID = "driftcode_connection_persistence"
    private const val NOTIFICATION_ID = 42420
    private const val ACTION_DISCONNECT_ALL = "app.tauri.connectionpersistence.action.DISCONNECT_ALL"
  }
}
