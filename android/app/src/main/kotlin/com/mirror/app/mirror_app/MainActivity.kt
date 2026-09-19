package com.mirror.app.mirror_app

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack
import android.os.Build
import android.util.Log
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

class MainActivity : FlutterActivity() {
    private val channelName = "com.vrv.desk/audio"
    private var audioTrack: AudioTrack? = null
    private var isMuted: Boolean = false
    private var currentSampleRate: Int = 0
    private var currentChannels: Int = 0
    private val audioExecutor: ExecutorService = Executors.newSingleThreadExecutor()

    companion object {
        private const val TAG = "AudioTrackNative"
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, channelName).setMethodCallHandler { call, result ->
            when (call.method) {
                "init" -> {
                    val sampleRate = call.argument<Int>("sampleRate") ?: 48000
                    val channels = call.argument<Int>("channels") ?: 2
                    val success = initAudioTrack(sampleRate, channels)
                    result.success(success)
                }
                "write" -> {
                    val data = call.argument<ByteArray>("data")
                    if (data != null) {
                        audioExecutor.execute {
                            writeAudioData(data)
                        }
                        result.success(true)
                    } else {
                        result.error("INVALID_ARGUMENT", "Audio data byte array is null", null)
                    }
                }
                "setMuted" -> {
                    val muted = call.argument<Boolean>("muted") ?: false
                    setMutedState(muted)
                    result.success(true)
                }
                "stop" -> {
                    stopAudioTrack()
                    result.success(true)
                }
                else -> {
                    result.notImplemented()
                }
            }
        }
    }

    @Synchronized
    private fun initAudioTrack(sampleRate: Int, channels: Int): Boolean {
        try {
            if (audioTrack != null && currentSampleRate == sampleRate && currentChannels == channels) {
                if (audioTrack?.playState != AudioTrack.PLAYSTATE_PLAYING) {
                    audioTrack?.play()
                }
                return true
            }

            stopAudioTrack()

            val channelConfig = if (channels == 1) {
                AudioFormat.CHANNEL_OUT_MONO
            } else {
                AudioFormat.CHANNEL_OUT_STEREO
            }

            val minBufferSize = AudioTrack.getMinBufferSize(
                sampleRate,
                channelConfig,
                AudioFormat.ENCODING_PCM_16BIT
            )

            if (minBufferSize <= 0) {
                Log.e(TAG, "Invalid minBufferSize: $minBufferSize for rate=$sampleRate channels=$channels")
                return false
            }

            val bufferSize = minBufferSize * 2

            val attributes = AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_MEDIA)
                .setContentType(AudioAttributes.CONTENT_TYPE_UNKNOWN)
                .build()

            val format = AudioFormat.Builder()
                .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                .setSampleRate(sampleRate)
                .setChannelMask(channelConfig)
                .build()

            val track = AudioTrack.Builder()
                .setAudioAttributes(attributes)
                .setAudioFormat(format)
                .setBufferSizeInBytes(bufferSize)
                .setTransferMode(AudioTrack.MODE_STREAM)
                .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
                .build()

            if (isMuted) {
                track.setVolume(0.0f)
            } else {
                track.setVolume(1.0f)
            }

            track.play()
            audioTrack = track
            currentSampleRate = sampleRate
            currentChannels = channels
            Log.i(TAG, "AudioTrack initialized successfully (sampleRate=$sampleRate, channels=$channels, bufferSize=$bufferSize)")
            return true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize AudioTrack", e)
            audioTrack = null
            return false
        }
    }

    @Synchronized
    private fun writeAudioData(data: ByteArray) {
        val track = audioTrack ?: return
        try {
            if (track.playState != AudioTrack.PLAYSTATE_PLAYING) {
                track.play()
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                track.write(data, 0, data.size, AudioTrack.WRITE_NON_BLOCKING)
            } else {
                track.write(data, 0, data.size)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error writing audio chunk", e)
        }
    }

    @Synchronized
    private fun setMutedState(muted: Boolean) {
        isMuted = muted
        val track = audioTrack ?: return
        try {
            val vol = if (muted) 0.0f else 1.0f
            track.setVolume(vol)
        } catch (e: Exception) {
            Log.e(TAG, "Error setting volume/mute state", e)
        }
    }

    @Synchronized
    private fun stopAudioTrack() {
        try {
            audioTrack?.let { track ->
                if (track.playState == AudioTrack.PLAYSTATE_PLAYING) {
                    track.pause()
                    track.flush()
                }
                track.release()
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error stopping AudioTrack", e)
        } finally {
            audioTrack = null
            currentSampleRate = 0
            currentChannels = 0
        }
    }

    override fun onDestroy() {
        stopAudioTrack()
        audioExecutor.shutdown()
        super.onDestroy()
    }
}
