//! Non-blocking AI inference tracing (`ZAROXI_AI_TRACE`).
//!
//! AI requests run on the tokio runtime, off the render thread. To surface
//! their latency in the per-frame `ZAROXI_PERF_TRACE` stream without blocking
//! rendering, async AI tasks push [`AiTraceEvent`]s into an unbounded
//! [`tokio::sync::mpsc`] channel. The render loop owns the [`AiTraceReceiver`]
//! and drains it once per frame with [`AiTraceReceiver::drain_to_trace`]
//! (a non-blocking `try_recv` loop).

use std::time::Duration;

/// One AI latency event produced by an async AI task.
#[derive(Clone, Debug, PartialEq)]
pub enum AiTraceEvent {
    /// Context-window / prompt assembly finished.
    PromptBuilt {
        /// Milliseconds spent assembling the prompt.
        ms: f32,
    },
    /// Request dispatched to the backend (marker; carries no duration).
    RequestSent,
    /// First streamed token arrived, measured from request-sent.
    FirstToken {
        /// Milliseconds from request-sent to first token.
        ms: f32,
    },
    /// Stream finished, measured from request-sent.
    StreamComplete {
        /// Milliseconds from request-sent to stream completion.
        ms: f32,
        /// Number of tokens streamed.
        tokens: usize,
        /// Throughput in tokens/second over the stream duration.
        tokens_per_sec: f32,
    },
    /// AI diff/suggestion applied to the buffer.
    Applied {
        /// Milliseconds spent applying the result to buffer state.
        ms: f32,
    },
}

impl AiTraceEvent {
    /// Compute tokens/sec for a [`AiTraceEvent::StreamComplete`].
    pub fn throughput(tokens: usize, dur: Duration) -> f32 {
        let secs = dur.as_secs_f32();
        if secs > 0.0 { tokens as f32 / secs } else { 0.0 }
    }

    /// Render the canonical `ZAROXI_AI_TRACE` line for this event.
    pub fn format_line(&self) -> String {
        match self {
            AiTraceEvent::PromptBuilt { ms } => {
                format!("ZAROXI_AI_TRACE: ai_prompt_build_ms={ms:.2}")
            }
            AiTraceEvent::RequestSent => "ZAROXI_AI_TRACE: ai_request_sent=1".to_string(),
            AiTraceEvent::FirstToken { ms } => {
                format!("ZAROXI_AI_TRACE: ai_first_token_ms={ms:.2}")
            }
            AiTraceEvent::StreamComplete { ms, tokens, tokens_per_sec } => {
                format!(
                    "ZAROXI_AI_TRACE: ai_stream_complete_ms={ms:.2} ai_tokens={tokens} ai_tokens_per_sec={tokens_per_sec:.1}"
                )
            }
            AiTraceEvent::Applied { ms } => format!("ZAROXI_AI_TRACE: ai_apply_ms={ms:.2}"),
        }
    }
}

/// Whether `ZAROXI_AI_TRACE=1` is set, or the umbrella `ZAROXI_PERF_TRACE=1`.
pub fn ai_trace_enabled() -> bool {
    matches!(std::env::var("ZAROXI_AI_TRACE").as_deref(), Ok("1"))
        || matches!(std::env::var("ZAROXI_PERF_TRACE").as_deref(), Ok("1"))
}

/// Cloneable sender handle given to async AI tasks. Sending never blocks and
/// never errors at the call site (a dropped receiver is silently ignored).
#[derive(Clone, Debug)]
pub struct AiTracer {
    tx: tokio::sync::mpsc::UnboundedSender<AiTraceEvent>,
}

/// Receiver half owned by the render loop.
#[derive(Debug)]
pub struct AiTraceReceiver {
    rx: tokio::sync::mpsc::UnboundedReceiver<AiTraceEvent>,
}

impl AiTracer {
    /// Create a connected `(sender, receiver)` pair.
    pub fn channel() -> (AiTracer, AiTraceReceiver) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (AiTracer { tx }, AiTraceReceiver { rx })
    }

    /// Push an event without blocking. A dropped receiver is ignored.
    pub fn emit(&self, event: AiTraceEvent) {
        let _ = self.tx.send(event);
    }
}

impl AiTraceReceiver {
    /// Drain all currently-queued events (non-blocking), printing each as a
    /// `ZAROXI_AI_TRACE` line when tracing is enabled. Returns the number drained.
    /// Intended to be called once per frame from the render loop.
    pub fn drain_to_trace(&mut self) -> usize {
        let mut count = 0;
        while let Ok(event) = self.rx.try_recv() {
            if ai_trace_enabled() {
                eprintln!("{}", event.format_line());
            }
            count += 1;
        }
        count
    }

    /// Drain all queued events into a `Vec` (non-blocking) without printing —
    /// used by the dashboard to summarise recent AI activity.
    pub fn drain(&mut self) -> Vec<AiTraceEvent> {
        let mut out = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            out.push(event);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throughput_is_tokens_over_seconds() {
        let tp = AiTraceEvent::throughput(100, Duration::from_secs(2));
        assert!((tp - 50.0).abs() < 1e-3);
        assert_eq!(AiTraceEvent::throughput(10, Duration::ZERO), 0.0);
    }

    #[test]
    fn format_lines() {
        assert_eq!(
            AiTraceEvent::PromptBuilt { ms: 1.5 }.format_line(),
            "ZAROXI_AI_TRACE: ai_prompt_build_ms=1.50"
        );
        assert_eq!(
            AiTraceEvent::StreamComplete { ms: 200.0, tokens: 40, tokens_per_sec: 200.0 }
                .format_line(),
            "ZAROXI_AI_TRACE: ai_stream_complete_ms=200.00 ai_tokens=40 ai_tokens_per_sec=200.0"
        );
    }

    #[tokio::test]
    async fn channel_roundtrip_non_blocking() {
        let (tracer, mut rx) = AiTracer::channel();
        tracer.emit(AiTraceEvent::RequestSent);
        tracer.emit(AiTraceEvent::FirstToken { ms: 12.0 });
        let drained = rx.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0], AiTraceEvent::RequestSent);
    }
}
