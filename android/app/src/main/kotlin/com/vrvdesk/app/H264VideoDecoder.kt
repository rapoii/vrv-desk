package com.vrvdesk.app

import android.media.MediaCodec
import android.media.MediaFormat
import android.util.Log
import android.view.Surface
import java.nio.ByteBuffer

class H264VideoDecoder(private val surface: Surface) {
    private var codec: MediaCodec? = null
    private var isConfigured = false
    private var presentationTimeUs = 0L
    private var framesReceived = 0L
    private var framesRendered = 0L

    companion object {
        private const val TAG = "H264VideoDecoder"
    }

    fun init(width: Int, height: Int): Boolean {
        return try {
            release()
            val format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, width, height).apply {
                // Low-latency decoding flags
                setInteger(MediaFormat.KEY_LOW_LATENCY, 1)
                setInteger(MediaFormat.KEY_PRIORITY, 0)
            }

            val decoder = MediaCodec.createDecoderByType(MediaFormat.MIMETYPE_VIDEO_AVC)
            decoder.configure(format, surface, null, 0)
            decoder.start()

            codec = decoder
            isConfigured = true
            presentationTimeUs = 0L
            framesReceived = 0L
            framesRendered = 0L
            Log.i(TAG, "H.264 hardware decoder initialized successfully for surface (${width}x${height})")
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize H.264 MediaCodec decoder: ${e.message}", e)
            isConfigured = false
            false
        }
    }

    fun decode(nalData: ByteArray) {
        val decoder = codec ?: return
        if (!isConfigured) return

        try {
            framesReceived++
            val inIndex = decoder.dequeueInputBuffer(10_000)
            if (inIndex >= 0) {
                val inputBuffer = decoder.getInputBuffer(inIndex)
                if (inputBuffer != null) {
                    inputBuffer.clear()
                    inputBuffer.put(nalData)
                    decoder.queueInputBuffer(inIndex, 0, nalData.size, presentationTimeUs, 0)
                    presentationTimeUs += 16_666L // ~60 fps
                }
            }

            val bufferInfo = MediaCodec.BufferInfo()
            var outIndex = decoder.dequeueOutputBuffer(bufferInfo, 0)
            while (outIndex >= 0) {
                // true = render directly to the attached Surface
                decoder.releaseOutputBuffer(outIndex, true)
                framesRendered++
                if (framesRendered % 100L == 0L) {
                    Log.i(TAG, "H.264 decoded stats: inFrames=$framesReceived, renderedFrames=$framesRendered, lastNalSize=${nalData.size}")
                }
                outIndex = decoder.dequeueOutputBuffer(bufferInfo, 0)
            }
        } catch (e: Exception) {
            Log.w(TAG, "Error during H.264 frame decoding: ${e.message}")
        }
    }

    fun release() {
        try {
            isConfigured = false
            codec?.stop()
            codec?.release()
        } catch (e: Exception) {
            Log.w(TAG, "Error releasing H.264 decoder: ${e.message}")
        } finally {
            codec = null
        }
    }
}
